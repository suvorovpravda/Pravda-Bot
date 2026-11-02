//! Native desktop adapter around the same transport-neutral market runtime.
//! This module never binds an HTTP port, constructs an Axum router, or
//! changes the paper simulator's trading behavior.
use crate::{config::ServerConfig, history::HistoryQuery, runtime_policy, Auth};
use chrono::Utc;
use pravda_core::PravdaRuntime;
use pravda_store::{HistoryTick, PravdaStore};
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Arc;

/// A typed Tauri return value, serialized as the existing dashboard JSON
/// contract (not as a JSON-encoded string).
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct DesktopPayload(pub Value);

pub struct DesktopRuntime {
    runtime: Arc<PravdaRuntime>,
    stale_after: std::time::Duration,
}

impl DesktopRuntime {
    /// Initialize on startup, before opening the WebView. Always operate on
    /// the same PRAVDA_DB_PATH and legacy migration contract as the HTTP CLI.
    pub async fn start(auth: Arc<Auth>) -> Result<Self, Box<dyn std::error::Error>> {
        let config = ServerConfig::from_env()?;
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
        })
    }

    pub async fn market_snapshot(&self) -> DesktopPayload {
        DesktopPayload(self.runtime.market_snapshot(self.stale_after).await)
    }

    pub async fn alerts_snapshot(&self) -> DesktopPayload {
        DesktopPayload(self.runtime.alerts_snapshot().await)
    }

    pub fn metrics_snapshot(&self) -> DesktopPayload {
        DesktopPayload(self.runtime.metrics_snapshot())
    }

    pub fn fx_status(&self) -> DesktopPayload {
        DesktopPayload(json!({
            "status": "unavailable",
            "rates": {},
            "timestamp": null,
            "notice": "FX connector not migrated"
        }))
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
