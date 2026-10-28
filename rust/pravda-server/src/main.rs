use axum::extract::ws::{Message, WebSocket};
use axum::{
    extract::State,
    extract::WebSocketUpgrade,
    http::HeaderValue,
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use pravda_engine::{AlertGate, PaperMode, PaperSimulator};
use pravda_fetchers::types::{Exchange, Symbol, Tick};
use pravda_store::PravdaStore;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// État partagé entre les handlers
struct AppState {
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

async fn dashboard() -> impl IntoResponse {
    let html = include_str!("../../../dashboard.html");
    let mut response = Response::new(html.to_string());
    let headers = response.headers_mut();

    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; connect-src 'self ws://127.0.0.1:8080'; img-src 'self' data:;"),
    );
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
                    ex_map_inner
                        .insert("timestamp".to_string(), json!(tick.timestamp.to_rfc3339()));
                    entry.insert(
                        format!("{:?}", exchange).to_lowercase(),
                        json!(ex_map_inner),
                    );
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
    Json(json!({"rates": {}, "source": "Frankfurter", "timestamp": "2026-10-29"}))
}

async fn api_history(State(state): State<Arc<AppState>>) -> Json<Value> {
    let store = state.store.lock().await;
    let symbol = "BTC/USDT";
    match store.latest_ticks(symbol, 100) {
        Ok(rows) => {
            let ticks = rows
                .into_iter()
                .map(|(price, ts)| json!({"price": price, "ts": ts}))
                .collect::<Vec<_>>();
            Json(json!({"symbol": symbol, "ticks": ticks}))
        }
        Err(_) => Json(json!({"symbol": symbol, "ticks": [], "error": "store unavailable"})),
    }
}

async fn api_news() -> Json<Value> {
    Json(json!({"articles": [], "message": "News endpoint"}))
}

async fn api_alerts(State(state): State<Arc<AppState>>) -> Json<Value> {
    let alerts = state.alerts.read().await;
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

async fn api_metrics() -> Json<Value> {
    Json(json!({"uptime_seconds": 0.0, "memory_usage_mb": 0.0}))
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tick_tx.subscribe();

    let send_task = tokio::spawn(async move {
        while let Ok(tick) = rx.recv().await {
            if let Ok(json_str) = serde_json::to_string(&tick) {
                if sender.send(Message::Text(json_str)).await.is_err() {
                    break;
                }
            }
        }
    });

    let recv_task =
        tokio::spawn(async move { while let Some(Ok(_msg)) = receiver.next().await {} });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tick_tx, _) = pravda_fetchers::create_tick_channel();

    let paper_sim = PaperSimulator::new(PaperMode::Fixed, 1000.0, 10.0, 100.0, 0.05, 3600.0, 0.36);

    let store = PravdaStore::open("pravda.db")?;
    let store_arc = Arc::new(tokio::sync::Mutex::new(store));

    let state = Arc::new(AppState {
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
        let mut rx = state_clone.tick_tx.subscribe();
        const FEE_PERCENT: f64 = 0.36;
        const REARM_THRESHOLD: f64 = 0.005;

        while let Ok(tick) = rx.recv().await {
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
                let store_guard = state_clone.store.lock().await;
                let _ = store_guard.insert_tick(
                    &tick.symbol.to_string(),
                    tick.price,
                    &format!("{:?}", tick.exchange),
                    tick.timestamp,
                );
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
                    let signed_spread = PaperSimulator::signed_spread_percent(
                        kraken_tick.price,
                        binance_tick.price,
                    );
                    let gross_spread = signed_spread.abs();
                    let net_spread = gross_spread - FEE_PERCENT;
                    let profitable = net_spread > 0.0;
                    let now = Utc::now();

                    // AlertGate
                    let mut gates = state_clone.alert_gates.write().await;
                    let gate = gates
                        .entry(tick.symbol)
                        .or_insert_with(|| AlertGate::new(3.0));
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
        .route("/api/news", get(api_news))
        .route("/api/alerts", get(api_alerts))
        .route("/api/metrics", get(api_metrics))
        .route("/ws/ticks", get(ws_handler))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("🚀 Pravda Server listening on http://127.0.0.1:8080");
    println!("📡 WebSocket endpoint: ws://127.0.0.1:8080/ws/ticks");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
