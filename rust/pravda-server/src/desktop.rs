//! Native desktop adapter around the same transport-neutral market runtime.
//! This module never binds an HTTP port, constructs an Axum router, or
//! changes the paper simulator's trading behavior.
use crate::{config::ServerConfig, history::HistoryQuery, runtime_policy, Auth};
use chrono::Utc;
use pravda_core::{
    agent_contract::{PaperSymbol, PaperVenue},
    agent_pipeline::{evidence_from_native_market_snapshot, MarketEvidence},
    PravdaRuntime,
};
use pravda_store::{HistoryTick, PravdaStore};
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Arc;

/// A typed Tauri return value, serialized as the existing dashboard JSON
/// contract (not as a JSON-encoded string).
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct DesktopPayload(pub Value);

/// Coarse status for data provenance. No probe is implied: a receiver that
/// has not observed ticks cannot claim its WebSocket connection succeeded.
fn stream_state(offline: bool, enabled: bool, snapshot: &Value) -> &'static str {
    if offline {
        return "diagnostic_offline";
    }
    if !enabled {
        return "disabled_by_configuration";
    }
    let sources = snapshot["symbols"].as_object();
    let mut saw_observation = false;
    if let Some(sources) = sources {
        for source in sources.values().filter_map(Value::as_object) {
            for quote in source.values() {
                if quote["status"] == "live" {
                    return "observations_recent";
                }
                if quote.get("price").and_then(Value::as_f64).is_some() {
                    saw_observation = true;
                }
            }
        }
    }
    if saw_observation {
        "observations_stale"
    } else {
        "awaiting_first_observation"
    }
}

pub struct DesktopRuntime {
    runtime: Arc<PravdaRuntime>,
    stale_after: std::time::Duration,
    market_feeds_enabled: bool,
    database_path: String,
    // One client per runtime preserves the shared 1 req/s public-feed throttle.
    revolut_public: Option<pravda_fetchers::revolut_x::RevolutXPublicClient>,
    // No public market network calls are allowed in the one-click diagnostic.
    diagnostic_offline: bool,
}

impl DesktopRuntime {
    /// Initialize on startup, before opening the WebView. Always operate on
    /// the same PRAVDA_DB_PATH and legacy migration contract as the HTTP CLI.
    pub async fn start(auth: Arc<Auth>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = ServerConfig::from_env()?;
        let diagnostic_offline = std::env::var("PRAVDA_OFFLINE_TEST").as_deref() == Ok("true");
        // A diagnostic flag must override *all* inherited online settings.
        // Never start Kraken/Binance background tasks on an isolated test run.
        if diagnostic_offline {
            config.market_feeds_enabled = false;
        }
        if !auth.is_configured() {
            if let Ok(initial) = std::env::var("PRAVDA_ADMIN_PASSWORD") {
                let authority = auth.clone();
                tokio::task::spawn_blocking(move || authority.set_password(&initial))
                    .await?
                    .map_err(|_| "Cannot initialize local password verifier")?;
            }
        }
        let mut store = PravdaStore::open(&config.database_path)?;
        let source = std::env::var_os("PRAVDA_LEGACY_DB_PATH")
            .or_else(|| std::env::var_os("MARKET_HISTORY_DB"))
            .or_else(|| {
                let default = std::path::PathBuf::from("market_history.sqlite3");
                default.exists().then_some(default.into_os_string())
            });
        if let Some(path) = source {
            if path.is_empty() {
                return Err("Legacy SQLite source path cannot be empty".into());
            }
            let report = store.migrate_legacy_schema(std::path::Path::new(&path))?;
            tracing::info!(
                source_rows = report.source_rows,
                inserted = report.inserted,
                already_imported = report.already_imported,
                "Desktop legacy SQLite migration verified"
            );
        }

        let (ticks, _) = pravda_fetchers::create_tick_channel();
        let runtime = Arc::new(PravdaRuntime::new(store, ticks));
        runtime.start_market_streams(runtime_policy(&config)).await;
        Ok(Self {
            runtime,
            stale_after: config.stale_after,
            market_feeds_enabled: config.market_feeds_enabled,
            database_path: config.database_path.clone(),
            revolut_public: if diagnostic_offline {
                None
            } else {
                pravda_fetchers::revolut_x::RevolutXPublicClient::new().ok()
            },
            diagnostic_offline,
        })
    }

    pub async fn market_snapshot(&self) -> DesktopPayload {
        let mut payload = self.runtime.market_snapshot(self.stale_after).await;
        let state = stream_state(self.diagnostic_offline, self.market_feeds_enabled, &payload);
        if let Some(fields) = payload.as_object_mut() {
            fields.insert("stream_state".to_string(), json!(state));
        }
        DesktopPayload(payload)
    }

    /// Read the SAME trusted native feed cache used by the desktop, then
    /// strictly normalize one observed quote for MDA. This does NOT verify
    /// a portfolio, budget, margin or model answer and cannot permit an order.
    pub async fn native_agent_market_evidence(
        &self,
        symbol: PaperSymbol,
        venue: PaperVenue,
        max_quote_age_ms: u64,
        max_price_deviation_bps: u16,
    ) -> Result<MarketEvidence, &'static str> {
        if self.diagnostic_offline || !self.market_feeds_enabled {
            return Err("MDA_STREAM_UNAVAILABLE");
        }
        let payload = self.market_snapshot().await;
        let now_ms = u64::try_from(Utc::now().timestamp_millis())
            .map_err(|_| "MDA_QUOTE_CLOCK_INVALID")?;
        evidence_from_native_market_snapshot(
            &payload.0,
            symbol,
            venue,
            now_ms,
            max_quote_age_ms,
            max_price_deviation_bps,
        )
        .map_err(|error| error.code())
    }

    pub async fn alerts_snapshot(&self) -> DesktopPayload {
        DesktopPayload(self.runtime.alerts_snapshot().await)
    }

    /// Read-only, bounded diagnostic from the SAME resolved desktop DB.
    /// Reports only a code, never the path, SQLite rows or raw OS errors.
    pub async fn security_database_health_code(&self) -> &'static str {
        let path = self.database_path.clone();
        tokio::task::spawn_blocking(move || {
            pravda_store::doctor::inspect_database(std::path::Path::new(&path)).code()
        })
        .await
        .unwrap_or("database_unavailable")
    }

    pub fn metrics_snapshot(&self) -> DesktopPayload {
        DesktopPayload(self.runtime.metrics_snapshot())
    }

    /// Public FX references, not exchange execution quotes. No credentials.
    pub async fn fx_status(&self) -> DesktopPayload {
        if self.diagnostic_offline {
            return DesktopPayload(json!({"status":"unavailable",
                "notice":"DIAGNOSTIC_OFFLINE","rates":{}}));
        }
        DesktopPayload(pravda_fetchers::public_feeds::reference_fx().await)
    }

    /// Strict enum dispatch. Never turns caller text into an HTTP URL.
    pub async fn public_feed(&self, source: &str) -> DesktopPayload {
        if self.diagnostic_offline {
            return DesktopPayload(json!({"status":"unavailable",
                "notice":"DIAGNOSTIC_OFFLINE"}));
        }
        let payload = match source {
            "energy" => pravda_fetchers::public_feeds::energy_flows().await,
            "news" => pravda_fetchers::public_feeds::geopolitical_news().await,
            "revolut" => match self.revolut_public.as_ref() {
                Some(client) => match client.order_book("BTC-USD").await {
                    Ok(book) => json!({"status":"delayed","source":"Revolut X public",
                        "symbol":"BTC-USD","book":book,"notice":"Carnet public, aucun solde ni ordre réel."}),
                    Err(_) => json!({"status":"unavailable","source":"Revolut X public",
                        "notice":"PUBLIC_ORDER_BOOK_UNAVAILABLE"}),
                },
                None => json!({"status":"unavailable","notice":"NETWORK_UNAVAILABLE"}),
            },
            "sources" => json!({"sources":[
                {"name":"Kraken / Binance","status":"stale","detail":"Statut à vérifier dans le terminal marchés"},
                {"name":"BCE via Frankfurter","status":"stale","detail":"Référence quotidienne à demander depuis l'écran Forex"},
                {"name":"ENTSOG","status":"stale","detail":"Observations publiques à demander depuis l'écran Énergie"},
                {"name":"GDELT","status":"stale","detail":"Articles rapportés à consulter depuis Veille"},
                {"name":"Revolut X","status":"stale","detail":"Carnet public uniquement ; aucun ordre réel"},
                {"name":"AIS","status":"unavailable","detail":"Source AIS autorisée ou clé serveur nécessaire"}
            ]}),
            _ => json!({"status":"unavailable","notice":"UNKNOWN_PUBLIC_SOURCE"}),
        };
        DesktopPayload(payload)
    }

    pub async fn history_recent(&self) -> Result<DesktopPayload, &'static str> {
        let rows = self
            .runtime
            .history_recent()
            .await
            .map_err(|_| "STORAGE_FAILURE")?;
        Ok(DesktopPayload(json!({
            "symbol": "BTC/USDT",
            "ticks": rows.into_iter().map(|(price, ts)| json!({"price": price, "ts": ts})).collect::<Vec<_>>()
        })))
    }

    /// Bounded by the same 10,001-row database query and 10,000-row response
    /// limit as Axum. Enumerated input rejects arbitrary SQL and huge limits.
    async fn history_rows(
        &self,
        query: HistoryQuery,
    ) -> Result<(Vec<HistoryTick>, bool), &'static str> {
        let store = self.runtime.store.clone();
        let now = Utc::now();
        let mut rows = tokio::task::spawn_blocking(move || {
            store.blocking_lock().history(
                query.symbol.name(),
                query.exchange.name(),
                now - query.period.duration(),
                now,
                10_001,
            )
        })
        .await
        .map_err(|_| "STORAGE_FAILURE")?
        .map_err(|_| "STORAGE_FAILURE")?;
        let truncated = rows.len() > 10_000;
        if truncated {
            rows.remove(0);
        }
        if rows.iter().any(|row| {
            !row.price.is_finite()
                || row.price <= 0.0
                || chrono::DateTime::parse_from_rfc3339(&row.timestamp).is_err()
        }) {
            return Err("STORAGE_FAILURE");
        }
        Ok((rows, truncated))
    }

    pub async fn history_query(&self, query: HistoryQuery) -> Result<DesktopPayload, &'static str> {
        let (rows, truncated) = self.history_rows(query).await?;
        Ok(DesktopPayload(json!({
            "symbol": query.symbol,
            "exchange": query.exchange,
            "period": query.period,
            "observations": rows.into_iter().map(|row| json!({
                "price": row.price,
                "timestamp": row.timestamp,
                "ticks": 1
            })).collect::<Vec<_>>(),
            "truncated": truncated
        })))
    }

    /// Returns CSV content only: there is no arbitrary filesystem or shell
    /// command in the Tauri capability. The renderer owns any download flow.
    pub async fn history_export_csv(&self, query: HistoryQuery) -> Result<String, &'static str> {
        let (rows, _) = self.history_rows(query).await?;
        let mut csv = String::from("timestamp,symbol,exchange,price\r\n");
        for row in rows {
            csv.push_str(&format!(
                "{},{},{},{}\r\n",
                row.timestamp,
                query.symbol.name(),
                query.exchange.name(),
                row.price
            ));
        }
        Ok(csv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{HistoryExchange, HistoryPeriod, HistorySymbol};

    #[test]
    fn feed_state_never_confuses_disabled_streams_with_missing_network_data() {
        let empty = json!({"symbols": {"BTC/USDT":{}, "ETH/USDT":{}}});
        assert_eq!(stream_state(true, true, &empty), "diagnostic_offline");
        assert_eq!(
            stream_state(false, false, &empty),
            "disabled_by_configuration"
        );
        assert_eq!(
            stream_state(false, true, &empty),
            "awaiting_first_observation"
        );
        let stale = json!({"symbols":{"BTC/USDT":{"Kraken":{"price":100.0,"status":"stale"}}}});
        assert_eq!(stream_state(false, true, &stale), "observations_stale");
        let live = json!({"symbols":{"BTC/USDT":{"Kraken":{"price":100.0,"status":"live"}}}});
        assert_eq!(stream_state(false, true, &live), "observations_recent");
    }

    #[test]
    fn typed_history_query_is_deny_unknown_and_strictly_enumerated() {
        assert!(serde_json::from_value::<HistoryQuery>(json!({
            "symbol": "BTC/USDT", "exchange": "Kraken", "period": "day"
        }))
        .is_ok());
        for input in [
            json!({"symbol": "DROP TABLE ticks", "exchange": "Kraken", "period": "day"}),
            json!({"symbol": "BTC/USDT", "exchange": "Other", "period": "day"}),
            json!({"symbol": "BTC/USDT", "exchange": "Kraken", "period": "forever"}),
            json!({"symbol": "BTC/USDT", "exchange": "Kraken", "period": "day", "limit": 999999}),
        ] {
            assert!(serde_json::from_value::<HistoryQuery>(input).is_err());
        }
        let q = HistoryQuery::default();
        assert!(matches!(q.symbol, HistorySymbol::Btc));
        assert!(matches!(q.exchange, HistoryExchange::Binance));
        assert!(matches!(q.period, HistoryPeriod::Day));
    }
}
