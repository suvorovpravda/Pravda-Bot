use axum::{
    extract::WebSocketUpgrade,
    http::HeaderValue,
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
    extract::State,
};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use pravda_fetchers::types::Tick;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// État partagé entre les handlers
struct AppState {
    tick_tx: broadcast::Sender<Tick>,
    last_ticks: RwLock<Vec<Tick>>,
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
    headers.insert("X-Content-Type-Options", HeaderValue::from_static("nosniff"));
    headers.insert("Referrer-Policy", HeaderValue::from_static("no-referrer-when-downgrade"));
    headers.insert("Content-Type", HeaderValue::from_static("text/html; charset=utf-8"));

    response
}

async fn api_market() -> Json<Value> {
    Json(json!({
        "symbols": {
            "BTC/USDT": {"binance": {"price": 0.0}, "kraken": {"price": 0.0}},
            "ETH/USDT": {"binance": {"price": 0.0}, "kraken": {"price": 0.0}},
            "SOL/USDT": {"binance": {"price": 0.0}, "kraken": {"price": 0.0}}
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn api_fx() -> Json<Value> {
    Json(json!({"rates": {}, "source": "Frankfurter", "timestamp": "2026-10-29"}))
}

async fn api_history() -> Json<Value> {
    Json(json!({"ticks": [], "message": "History endpoint - SQLite WAL"}))
}

async fn api_news() -> Json<Value> {
    Json(json!({"articles": [], "message": "News endpoint"}))
}

async fn api_alerts() -> Json<Value> {
    Json(json!({"alerts": [], "settings": {"mode": "paper"}}))
}

async fn api_metrics() -> Json<Value> {
    Json(json!({"uptime_seconds": 0.0, "memory_usage_mb": 0.0}))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
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

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(_msg)) = receiver.next().await {}
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tick_tx, _) = pravda_fetchers::create_tick_channel();
    
    let state = Arc::new(AppState {
        tick_tx: tick_tx.clone(),
        last_ticks: RwLock::new(Vec::new()),
    });

    let symbols = vec![
        pravda_fetchers::types::Symbol::BTCUSDT,
        pravda_fetchers::types::Symbol::ETHUSDT,
        pravda_fetchers::types::Symbol::SOLUSDT,
    ];

    // Lance les fetchers en arrière-plan
    let tx_clone = tick_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = pravda_fetchers::run_fetchers(tx_clone, symbols).await {
            tracing::error!("Fetchers error: {}", e);
        }
    });

    // Stocke les derniers ticks
    let state_clone = state.clone();
    tokio::spawn(async move {
        let mut rx = state_clone.tick_tx.subscribe();
        while let Ok(tick) = rx.recv().await {
            let mut ticks = state_clone.last_ticks.write().await;
            ticks.push(tick);
            if ticks.len() > 1000 {
                ticks.remove(0);
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
