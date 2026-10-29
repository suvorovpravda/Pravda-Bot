use axum::extract::ws::{Message, WebSocket};
mod config;
mod history;
mod http_security;
use axum::{
    extract::State,
    extract::WebSocketUpgrade,
    http::HeaderValue,
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use chrono::Utc;
use config::ServerConfig;
use futures_util::{SinkExt, StreamExt};
use pravda_engine::{AlertGate, PaperMode, PaperSimulator};
use pravda_fetchers::types::{Exchange, Symbol, Tick};
use pravda_market_core::health::{assess_market_data, HealthStatus};
use pravda_market_core::{calculate_spread, DEFAULT_FEE_RATE};
use pravda_store::PravdaStore;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env; // added for env var
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, RwLock};

/// État partagé entre les handlers
struct AppState {
    config: ServerConfig,
    started_at: Instant,
    tick_tx: broadcast::Sender<Tick>,
    last_ticks: RwLock<Vec<Tick>>,
    prices: RwLock<HashMap<Symbol, HashMap<Exchange, Tick>>>,
    alert_gates: RwLock<HashMap<Symbol, AlertGate>>,
    paper_sim: RwLock<PaperSimulator>,
    alerts: RwLock<Vec<AlertEvent>>,
    store: Arc<tokio::sync::Mutex<PravdaStore>>,
}

#[derive(Debug, Clone)]
struct AlertEvent {
    symbol: String,
    message: String,
    timestamp: chrono::DateTime<Utc>,
    spread_percent: f64,
}

async fn dashboard(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let html = include_str!("../../../dashboard.html");
    let socket_url = format!("ws://127.0.0.1:{}/ws/ticks", state.config.port);
    let mut response = Response::new(html.replace("ws://127.0.0.1:8080/ws/ticks", &socket_url));
    let headers = response.headers_mut();

    // Determine port from env var for CSP
    let port: u16 = std::env::var("DASHBOARD_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);
    let csp = format!("default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; connect-src 'self ws://127.0.0.1:{port}'; img-src 'self' data:;");
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_str(&csp).unwrap(),    );
    headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));
    headers.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        "Referrer-Policy",
        HeaderValue::from_static("no-referrer-when-downgrade"),
    );
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("text/html; charset=utf-8"),
    );

    response
}

async fn api_market(State(state): State<Arc<AppState>>) -> Json<Value> {
    let prices = state.prices.read().await;
    let mut symbols_map = serde_json::Map::new();
    for symbol in [Symbol::BTCUSDT, Symbol::ETHUSDT, Symbol::SOLUSDT] {
        let mut entry = serde_json::Map::new();
        if let Some(ex_map) = prices.get(&symbol) {
            for exchange in [Exchange::Binance, Exchange::Kraken] {
                if let Some(tick) = ex_map.get(&exchange) {
                    let mut ex_map_inner = serde_json::Map::new();
                    ex_map_inner.insert("price".to_string(), json!(tick.price));
                    let age: f64 = tick.observed_at.elapsed().as_secs_f64();
                    ex_map_inner.insert("age_seconds".to_string(), json!(age));
                    ex_map_inner.insert(
                        "status".to_string(),
                        json!(if age <= state.config.stale_after.as_secs_f64() {
                            "live"
                        } else {
                            "stale"
                        }),
                    );
                    ex_map_inner
                        .insert("timestamp".to_string(), json!(tick.timestamp.to_rfc3339()));
                    entry.insert(format!("{:?}", exchange), json!(ex_map_inner));
                }
            }
        }
        symbols_map.insert(symbol.to_string(), json!(entry));
    }
    Json(json!({
        "symbols": symbols_map,
        "timestamp": Utc::now().to_rfc3339()
    }))
}

async fn api_fx() -> Json<Value> {
    Json(
        json!({"status": "unavailable", "rates": {}, "timestamp": null, "notice": "FX connector not migrated"}),
    )
}

async fn api_history(State(state): State<Arc<AppState>>) -> Json<Value> {
    let symbol = "BTC/USDT";
    let store = state.store.clone();
    match tokio::task::spawn_blocking(move || store.blocking_lock().latest_ticks(symbol, 100)).await
    {
        Ok(Ok(rows)) => {
            let ticks = rows
                .into_iter()
                .map(|(price, ts)| json!({"price": price, "ts": ts}))
                .collect::<Vec<_>>();
            Json(json!({"symbol": symbol, "ticks": ticks}))
        }
        _ => Json(json!({"symbol": symbol, "ticks": [], "error": "store unavailable"})),
    }
}

async fn api_news() -> Json<Value> {
    Json(json!({"articles": [], "message": "News endpoint"}))
}

async fn api_alerts(State(state): State<Arc<AppState>>) -> Json<Value> {
    // Release alerts before waiting for paper_sim: the writer takes the
    // simulator lock first and then appends alerts.
    let alerts = state.alerts.read().await.clone();
    let sim = state.paper_sim.read().await;
    Json(json!({
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
    }))
}

async fn api_metrics(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(
        json!({"uptime_seconds": state.started_at.elapsed().as_secs_f64(), "memory_usage_mb": null, "cpu_percent": null, "status": "partial", "pid": std::process::id()}),
    )
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tick_tx.subscribe();

    let mut send_task = tokio::spawn(async move {
        loop {
            let tick = match rx.recv().await {
                Ok(tick) => tick,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            };
            if let Ok(json_str) = serde_json::to_string(&tick) {
                if sender.send(Message::Text(json_str)).await.is_err() {
                    break;
                }
            }
        }
    });

    let mut recv_task =
        tokio::spawn(async move { while let Some(Ok(_msg)) = receiver.next().await {} });

    tokio::select! {
        _ = &mut send_task => { recv_task.abort(); let _ = recv_task.await; },
        _ = &mut recv_task => { send_task.abort(); let _ = send_task.await; },
    }
}

//
pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig::from_env()?;
    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    // Bind before opening the database or starting any market connections.
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let (tick_tx, _) = pravda_fetchers::create_tick_channel();

    let paper_sim = PaperSimulator::new(PaperMode::Fixed, 1000.0, 10.0, 100.0, 0.05, 3600.0, 0.36);

    let store = PravdaStore::open(&config.database_path)?;
    let store_arc = Arc::new(tokio::sync::Mutex::new(store));

    let state = Arc::new(AppState {
        config,
        started_at: Instant::now(),
        tick_tx: tick_tx.clone(),
        last_ticks: RwLock::new(Vec::new()),
        prices: RwLock::new(HashMap::new()),
        alert_gates: RwLock::new(HashMap::new()),
        paper_sim: RwLock::new(paper_sim),
        alerts: RwLock::new(Vec::new()),
        store: store_arc,
    });

    let symbols = vec![
        pravda_fetchers::types::Symbol::BTCUSDT,
        pravda_fetchers::types::Symbol::ETHUSDT,
        pravda_fetchers::types::Symbol::SOLUSDT,
    ];

    // Initialiser les alert gates
    {
        let mut gates = state.alert_gates.write().await;
        for s in &symbols {
            gates.insert(*s, AlertGate::new(3.0));
        }
    }

    // Subscribe before starting producers so startup frames cannot be lost.
    let mut rx = state.tick_tx.subscribe();
    // Lance les fetchers en arrière-plan
    let tx_clone = tick_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = pravda_fetchers::run_fetchers(tx_clone, symbols.clone()).await {
            tracing::error!("Fetchers error: {}", e);
        }
    });

    // Traitement temps réel des ticks
    let state_clone = state.clone();
    tokio::spawn(async move {
        const REARM_THRESHOLD: f64 = 0.005;

        loop {
            let tick = match rx.recv().await {
                Ok(tick) => tick,
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    tracing::warn!(count, "Ticks lost: invalidate pending market state");
                    state_clone.prices.write().await.clear();
                    state_clone.alert_gates.write().await.clear();
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            };
            if !tick.price.is_finite() || tick.price <= 0.0 {
                continue;
            }
            // Mettre à jour les prix
            {
                let mut prices = state_clone.prices.write().await;
                let ex_map = prices.entry(tick.symbol).or_insert_with(HashMap::new);
                ex_map.insert(tick.exchange, tick.clone());
            }

            // Stocker historique en mémoire
            {
                let mut ticks = state_clone.last_ticks.write().await;
                ticks.push(tick.clone());
                if ticks.len() > 1000 {
                    ticks.remove(0);
                }
            }

            // Persister en SQLite
            {
                let store = state_clone.store.clone();
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
            let prices_read = state_clone.prices.read().await;
            if let Some(ex_map) = prices_read.get(&tick.symbol) {
                if let (Some(kraken), Some(binance)) = (
                    ex_map.get(&Exchange::Kraken),
                    ex_map.get(&Exchange::Binance),
                ) {
                    let kraken_tick = kraken.clone();
                    let binance_tick = binance.clone();
                    let Some(spread) = configured_spread(
                        &kraken_tick,
                        &binance_tick,
                        Instant::now(),
                        &state_clone.config,
                    ) else {
                        state_clone.alert_gates.write().await.remove(&tick.symbol);
                        continue;
                    };
                    let gross_spread = spread.gross_percent();
                    let net_spread = spread.net_percent();
                    let profitable = net_spread > 0.0;
                    let now = Utc::now();

                    // AlertGate
                    let mut gates = state_clone.alert_gates.write().await;
                    let gate = gates
                        .entry(tick.symbol)
                        .or_insert_with(|| AlertGate::new(3.0));
                    if configured_spread(
                        &kraken_tick,
                        &binance_tick,
                        Instant::now(),
                        &state_clone.config,
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
                        let mut alerts = state_clone.alerts.write().await;
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
                    let mut sim = state_clone.paper_sim.write().await;
                    if configured_spread(
                        &kraken_tick,
                        &binance_tick,
                        Instant::now(),
                        &state_clone.config,
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
                        let mut alerts = state_clone.alerts.write().await;
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

    let app = Router::new()
        .route("/", get(dashboard))
        .route("/api/market", get(api_market))
        .route("/api/fx", get(api_fx))
        .route("/api/history", get(api_history))
        .route("/api/market/history", get(history::json_history))
        .route("/api/market/history.csv", get(history::csv_history))
        .route("/api/news", get(api_news))
        .route("/api/alerts", get(api_alerts))
        .route("/api/metrics", get(api_metrics))
        .route("/ws/ticks", get(ws_handler))
        .layer(axum::middleware::from_fn_with_state(
            state.config.port,
            http_security::local_only,
        ))
        .with_state(state);


    // Determine port from env var or default 8080
    let port: u16 = std::env::var("DASHBOARD_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("🚀 Pravda Server listening on http://127.0.0.1:{port}");
    println!("📡 WebSocket endpoint: ws://127.0.0.1:{port}/ws/ticks");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Fail closed before alerting or simulation; fees are a fraction internally.
fn configured_spread(
    kraken: &Tick,
    binance: &Tick,
    now: Instant,
    config: &ServerConfig,
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
        config.stale_after,
        config.max_skew,
    )
    .ok()?;
    if health.status != HealthStatus::Ok {
        return None;
    }
    let spread = calculate_spread(kraken.price, binance.price, DEFAULT_FEE_RATE).ok()?;
    if spread.gross_percent() > config.max_spread_percent {
        return None;
    }
    Some(spread)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    fn actionable_spread(k: &Tick, b: &Tick, now: Instant) -> Option<pravda_market_core::Spread> {
        configured_spread(k, b, now, &ServerConfig::default())
    }
    fn quote(exchange: Exchange, price: f64, observed_at: Instant) -> Tick {
        Tick {
            symbol: Symbol::BTCUSDT,
            price,
            exchange,
            timestamp: Utc::now(),
            observed_at,
        }
    }
    #[test]
    fn guard_rejects_stale_skew_anomalies_and_invalid_numbers() {
        let now: Instant = Instant::now();
        let b: Tick = quote(Exchange::Binance, 100.0, now);
        for price in [0.0, -1.0, f64::NAN, f64::INFINITY, 106.0] {
            assert!(actionable_spread(&quote(Exchange::Kraken, price, now), &b, now).is_none());
        }
        for seconds in [3_u64, 20_u64] {
            assert!(actionable_spread(
                &quote(Exchange::Kraken, 101.0, now - Duration::from_secs(seconds)),
                &b,
                now
            )
            .is_none());
        }
        let valid = actionable_spread(&quote(Exchange::Kraken, 101.0, now), &b, now).unwrap();
        assert!((valid.net_percent() - 0.64).abs() < 1e-12);
        assert!(actionable_spread(
            &quote(Exchange::Kraken, 101.0, now),
            &b,
            now + Duration::from_secs(5)
        )
        .is_none());
    }
    #[test]
    fn configured_thresholds_control_actual_spread_guard() {
        let now = Instant::now();
        let k = quote(Exchange::Kraken, 101.0, now);
        let b = quote(Exchange::Binance, 100.0, now);
        let strict = ServerConfig {
            max_spread_percent: 0.5,
            ..ServerConfig::default()
        };
        assert!(configured_spread(&k, &b, now, &strict).is_none());
        let short = ServerConfig {
            stale_after: Duration::from_secs(1),
            ..ServerConfig::default()
        };
        assert!(configured_spread(&k, &b, now + Duration::from_secs(2), &short).is_none());
        assert!(configured_spread(
            &k,
            &b,
            now + Duration::from_secs(2),
            &ServerConfig::default()
        )
        .is_some());
    }
}
