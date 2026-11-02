//! Transport-agnostic application state and market processing.
//! No Axum, Tauri, HTTP cookie or desktop process logic is allowed here.
use chrono::Utc;
use pravda_engine::{AlertGate, PaperMode, PaperSimulator};
use pravda_fetchers::types::{Exchange, Symbol, Tick};
use pravda_market_core::health::{assess_market_data, HealthStatus};
use pravda_market_core::{calculate_spread, DEFAULT_FEE_RATE};
use pravda_store::{PravdaStore, StoreError};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};

/// All market thresholds are injected by the caller, not read from environment.
#[derive(Debug, Clone, Copy)]
pub struct RuntimePolicy {
    pub stale_after: Duration,
    pub max_skew: Duration,
    pub max_spread_percent: f64,
    pub market_feeds_enabled: bool,
}

/// The running market/paper engine: shared by HTTP diagnostic adapter today,
/// and by IPC commands when PR-04/05 introduce them.
pub struct PravdaRuntime {
    pub started_at: Instant,
    pub tick_tx: broadcast::Sender<Tick>,
    pub last_ticks: RwLock<Vec<Tick>>,
    pub prices: RwLock<HashMap<Symbol, HashMap<Exchange, Tick>>>,
    pub alert_gates: RwLock<HashMap<Symbol, AlertGate>>,
    pub paper_sim: RwLock<PaperSimulator>,
    pub alerts: RwLock<Vec<AlertEvent>>,
    pub store: Arc<tokio::sync::Mutex<PravdaStore>>,
}

#[derive(Debug, Clone)]
pub struct AlertEvent {
    pub symbol: String,
    pub message: String,
    pub timestamp: chrono::DateTime<Utc>,
    pub spread_percent: f64,
}

impl PravdaRuntime {
    /// Preserve the established rusqlite connection and WAL schema; no SQLx
    /// migration or additional connection pools are introduced.
    pub fn new(store: PravdaStore, tick_tx: broadcast::Sender<Tick>) -> Self {
        Self {
            started_at: Instant::now(),
            tick_tx,
            last_ticks: RwLock::new(Vec::new()),
            prices: RwLock::new(HashMap::new()),
            alert_gates: RwLock::new(HashMap::new()),
            paper_sim: RwLock::new(PaperSimulator::new(
                PaperMode::Fixed,
                1000.0,
                10.0,
                100.0,
                0.05,
                3600.0,
                0.36,
            )),
            alerts: RwLock::new(Vec::new()),
            store: Arc::new(tokio::sync::Mutex::new(store)),
        }
    }

    /// Business projection stays independent from HTTP or Tauri IPC DTOs.
    pub async fn market_snapshot(&self, stale_after: Duration) -> Value {
        let prices = self.prices.read().await;
        let mut symbols_map = serde_json::Map::new();
        for symbol in [Symbol::BTCUSDT, Symbol::ETHUSDT, Symbol::SOLUSDT] {
            let mut entry = serde_json::Map::new();
            if let Some(ex_map) = prices.get(&symbol) {
                for exchange in [Exchange::Binance, Exchange::Kraken] {
                    if let Some(tick) = ex_map.get(&exchange) {
                        let mut quote = serde_json::Map::new();
                        quote.insert("price".to_owned(), json!(tick.price));
                        let age = tick.observed_at.elapsed().as_secs_f64();
                        quote.insert("age_seconds".to_owned(), json!(age));
                        quote.insert("status".to_owned(), json!(if age <= stale_after.as_secs_f64() {
                            "live"
                        } else {
                            "stale"
                        }));
                        quote.insert("timestamp".to_owned(), json!(tick.timestamp.to_rfc3339()));
                        entry.insert(format!("{:?}", exchange), Value::Object(quote));
                    }
                }
            }
            symbols_map.insert(symbol.to_string(), Value::Object(entry));
        }
        json!({"symbols": symbols_map, "timestamp": Utc::now().to_rfc3339()})
    }

    /// Take the alert snapshot *before* the simulator lock to preserve the
    /// existing lock-order guarantee and avoid writer/reader deadlocks.
    pub async fn alerts_snapshot(&self) -> Value {
        let alerts = self.alerts.read().await.clone();
        let sim = self.paper_sim.read().await;
        json!({
            "alerts": alerts.iter().map(|a| json!({
                "symbol": a.symbol,
                "message": a.message,
                "timestamp": a.timestamp.to_rfc3339(),
                "spread_percent": a.spread_percent
            })).collect::<Vec<_>>(),
            "paper": {
                "equity_usdt": sim.equity_usdt(),
                "cash_usdt": sim.cash_usdt,
                "realized_pnl_usdt": sim.realized_pnl_usdt,
                "positions": sim.positions.len(),
                "completed_positions": sim.completed_positions
            },
            "settings": {"mode": "paper"}
        })
    }

    pub fn metrics_snapshot(&self) -> Value {
        json!({
            "uptime_seconds": self.started_at.elapsed().as_secs_f64(),
            "memory_usage_mb": null,
            "cpu_percent": null,
            "status": "partial",
            "pid": std::process::id()
        })
    }

    /// Bounded database access executed off the async event loop.
    pub async fn history_recent(&self) -> Result<Vec<(f64, String)>, StoreError> {
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || store.blocking_lock().latest_ticks("BTC/USDT", 100))
            .await
            .map_err(|_| StoreError::InvalidTick)?
    }

    /// Producers subscribe and process through the core; Axum isn't involved.
    /// PR-06 will add explicit lifecycle/shutdown ownership without changing
    /// the existing background-task behavior in this extraction.
    pub async fn start_market_streams(self: &Arc<Self>, policy: RuntimePolicy) {
        let symbols = vec![
            pravda_fetchers::types::Symbol::BTCUSDT,
            pravda_fetchers::types::Symbol::ETHUSDT,
            pravda_fetchers::types::Symbol::SOLUSDT,
        ];

        // Initialiser les alert gates
        {
            let mut gates = self.alert_gates.write().await;
            for s in &symbols {
                gates.insert(*s, AlertGate::new(3.0));
            }
        }

        // Subscribe before starting producers so startup frames cannot be lost.
        let mut rx = self.tick_tx.subscribe();
        // The explicit offline mode is useful for local diagnostics and smoke tests.
        if policy.market_feeds_enabled {
            let tx_clone = tick_tx.clone();
            tokio::spawn(async move {
                if let Err(e) = pravda_fetchers::run_fetchers(tx_clone, symbols.clone()).await {
                    tracing::error!("Fetchers error: {}", e);
                }
            });
        }

        // Traitement temps réel des ticks
        let runtime = self.clone();
        tokio::spawn(async move {
            const REARM_THRESHOLD: f64 = 0.005;

            loop {
                let tick = match rx.recv().await {
                    Ok(tick) => tick,
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        tracing::warn!(count, "Ticks lost: invalidate pending market state");
                        runtime.prices.write().await.clear();
                        runtime.alert_gates.write().await.clear();
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                };
                if !tick.price.is_finite() || tick.price <= 0.0 {
                    continue;
                }
                // Mettre à jour les prix
                {
                    let mut prices = runtime.prices.write().await;
                    let ex_map = prices.entry(tick.symbol).or_insert_with(HashMap::new);
                    ex_map.insert(tick.exchange, tick.clone());
                }

                // Stocker historique en mémoire
                {
                    let mut ticks = runtime.last_ticks.write().await;
                    ticks.push(tick.clone());
                    if ticks.len() > 1000 {
                        ticks.remove(0);
                    }
                }

                // Persister en SQLite
                {
                    let store = runtime.store.clone();
                    let saved_tick = tick.clone();
                    let result = tokio::task::spawn_blocking(move || {
                        store.blocking_lock().insert_tick(
                            &saved_tick.symbol.to_string(),
                            saved_tick.price,
                            &format!("{:?}", saved_tick.exchange),
                            saved_tick.timestamp,
                        )
                    })
                    .await;
                    if !matches!(result, Ok(Ok(()))) {
                        tracing::error!("Tick persistence failed");
                    }
                }

                // Calcul spread si les deux exchanges présents
                let prices_read = runtime.prices.read().await;
                if let Some(ex_map) = prices_read.get(&tick.symbol) {
                    if let (Some(kraken), Some(binance)) = (
                        ex_map.get(&Exchange::Kraken),
                        ex_map.get(&Exchange::Binance),
                    ) {
                        let kraken_tick = kraken.clone();
                        let binance_tick = binance.clone();
                        let Some(spread) = guard_spread(
                            &kraken_tick,
                            &binance_tick,
                            Instant::now(),
                            &policy,
                        ) else {
                            runtime.alert_gates.write().await.remove(&tick.symbol);
                            continue;
                        };
                        let gross_spread = spread.gross_percent();
                        let net_spread = spread.net_percent();
                        let profitable = net_spread > 0.0;
                        let now = Utc::now();

                        // AlertGate
                        let mut gates = runtime.alert_gates.write().await;
                        let gate = gates
                            .entry(tick.symbol)
                            .or_insert_with(|| AlertGate::new(3.0));
                        if guard_spread(
                            &kraken_tick,
                            &binance_tick,
                            Instant::now(),
                            &policy,
                        )
                        .is_none()
                        {
                            gate.reset_candidate(&tick.symbol.to_string());
                            continue;
                        }
                        let activated = gate.update(
                            &tick.symbol.to_string(),
                            profitable,
                            gross_spread,
                            REARM_THRESHOLD,
                            now,
                        );

                        if activated {
                            let mut alerts = runtime.alerts.write().await;
                            alerts.push(AlertEvent {
                                symbol: tick.symbol.to_string(),
                                message: format!(
                                    "⚡ Alerte rentable {} spread {:.4}%",
                                    tick.symbol, gross_spread
                                ),
                                timestamp: now,
                                spread_percent: gross_spread,
                            });
                            if alerts.len() > 200 {
                                alerts.remove(0);
                            }
                        }

                        let opportunity_confirmed = gate.is_active(&tick.symbol.to_string());
                        drop(gates);
                        drop(prices_read);

                        // Paper simulator
                        let mut sim = runtime.paper_sim.write().await;
                        if guard_spread(
                            &kraken_tick,
                            &binance_tick,
                            Instant::now(),
                            &policy,
                        )
                        .is_none()
                        {
                            continue;
                        }
                        if let Some(action) = sim.update(
                            &tick.symbol.to_string(),
                            kraken_tick.price,
                            binance_tick.price,
                            opportunity_confirmed,
                            now,
                        ) {
                            let mut alerts = runtime.alerts.write().await;
                            alerts.push(AlertEvent {
                                symbol: tick.symbol.to_string(),
                                message: format!("Paper {} {}", tick.symbol, action),
                                timestamp: now,
                                spread_percent: gross_spread,
                            });
                            if alerts.len() > 200 {
                                alerts.remove(0);
                            }
                        }
                    }
                }
            }
        });

    }
}

/// Fail closed before paper simulation. Fees remain fractions internally.
pub fn guard_spread(
    kraken: &Tick,
    binance: &Tick,
    now: Instant,
    policy: &RuntimePolicy,
) -> Option<pravda_market_core::Spread> {
    if kraken.symbol != binance.symbol
        || kraken.exchange != Exchange::Kraken
        || binance.exchange != Exchange::Binance
    {
        return None;
    }
    let health = assess_market_data(
        Some(kraken.observed_at),
        Some(binance.observed_at),
        now,
        policy.stale_after,
        policy.max_skew,
    ).ok()?;
    if health.status != HealthStatus::Ok {
        return None;
    }
    let spread = calculate_spread(kraken.price, binance.price, DEFAULT_FEE_RATE).ok()?;
    if spread.gross_percent() > policy.max_spread_percent {
        return None;
    }
    Some(spread)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_policy() -> RuntimePolicy {
        RuntimePolicy {
            stale_after: Duration::from_secs(4),
            max_skew: Duration::from_secs(2),
            max_spread_percent: 5.0,
            market_feeds_enabled: false,
        }
    }
    fn quote(exchange: Exchange, price: f64, observed_at: Instant) -> Tick {
        Tick {
            symbol: Symbol::BTCUSDT, price, exchange,
            timestamp: Utc::now(), observed_at
        }
    }
    #[test]
    fn guard_preserves_stale_invalid_spread_and_fee_invariants() {
        let now = Instant::now();
        let binance = quote(Exchange::Binance, 100.0, now);
        let mut policy = test_policy();
        for price in [-1.0, 0.0, f64::NAN, f64::INFINITY, 106.0] {
            assert!(guard_spread(&quote(Exchange::Kraken, price, now), &binance, now, &policy).is_none());
        }
        let kraken = quote(Exchange::Kraken, 101.0, now);
        let spread = guard_spread(&kraken, &binance, now, &policy).unwrap();
        assert!((spread.net_percent() - 0.64).abs() < 1e-12);
        policy.max_spread_percent = 0.5;
        assert!(guard_spread(&kraken, &binance, now, &policy).is_none());
        policy.max_spread_percent = 5.0;
        assert!(guard_spread(&kraken, &binance, now + Duration::from_secs(5), &policy).is_none());
    }
    #[tokio::test]
    async fn snapshot_and_sqlite_stay_usable_without_http_or_tauri() {
        let store = PravdaStore::open(":memory:").unwrap();
        let (tx, _) = broadcast::channel(16);
        let runtime = PravdaRuntime::new(store, tx);
        let empty = runtime.market_snapshot(Duration::from_secs(4)).await;
        assert!(empty["symbols"]["BTC/USDT"].as_object().unwrap().is_empty());
        let now = Instant::now();
        runtime.prices.write().await.entry(Symbol::BTCUSDT).or_default()
            .insert(Exchange::Kraken, quote(Exchange::Kraken, 101.0, now));
        let snapshot = runtime.market_snapshot(Duration::from_secs(4)).await;
        assert_eq!(snapshot["symbols"]["BTC/USDT"]["Kraken"]["price"], 101.0);
        assert_eq!(snapshot["symbols"]["BTC/USDT"]["Kraken"]["status"], "live");
        let paper = runtime.alerts_snapshot().await;
        assert_eq!(paper["paper"]["cash_usdt"], 1000.0);
        assert!(runtime.history_recent().await.unwrap().is_empty());
        assert_eq!(runtime.metrics_snapshot()["status"], "partial");
    }
}
