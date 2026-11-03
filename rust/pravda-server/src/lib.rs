use axum::extract::ws::{Message, WebSocket};
mod auth;
pub use auth::{Auth, AuthError, PBKDF2_ITERATIONS};
mod config;
mod desktop;
mod history;
pub mod revolut_cli;
pub mod revolut_x_account;
pub mod secret_store;
pub use desktop::{DesktopPayload, DesktopRuntime};
pub use history::HistoryQuery as DesktopHistoryQuery;
mod http_security;
use axum::{
    extract::State,
    extract::WebSocketUpgrade,
    http::HeaderValue,
    response::{IntoResponse, Json, Response},
    routing::get,
    Extension, Router,
};
use config::ServerConfig;
use futures_util::{SinkExt, StreamExt};
use pravda_core::{PravdaRuntime, RuntimePolicy};
use pravda_store::PravdaStore;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;

/// HTTP adapter: no market processing or database ownership belongs to Axum.
struct AppState {
    config: ServerConfig,
    runtime: Arc<PravdaRuntime>,
    auth: Arc<auth::Auth>,
}

impl std::ops::Deref for AppState {
    type Target = PravdaRuntime;
    fn deref(&self) -> &Self::Target {
        self.runtime.as_ref()
    }
}

fn runtime_policy(config: &ServerConfig) -> RuntimePolicy {
    RuntimePolicy {
        stale_after: config.stale_after,
        max_skew: config.max_skew,
        max_spread_percent: config.max_spread_percent,
        market_feeds_enabled: config.market_feeds_enabled,
    }
}

async fn dashboard(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let html = include_str!("../../../dashboard.html");
    let mut response = Response::new(axum::body::Body::from(html));
    let headers = response.headers_mut();

    let port = state.config.port;
    let csp = format!("default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; connect-src 'self' ws://127.0.0.1:{port} ws://localhost:{port}; img-src 'self' data:; frame-ancestors 'none'; base-uri 'none';");
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_str(&csp).expect("static CSP and numeric port are valid header bytes"),
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
    Json(state.market_snapshot(state.config.stale_after).await)
}

async fn api_fx() -> Json<Value> {
    Json(
        json!({"status": "unavailable", "rates": {}, "timestamp": null, "notice": "FX connector not migrated"}),
    )
}

async fn api_history(State(state): State<Arc<AppState>>) -> Json<Value> {
    let symbol = "BTC/USDT";
    match state.history_recent().await {
        Ok(rows) => Json(json!({
            "symbol": symbol,
            "ticks": rows.into_iter().map(|(price, ts)| json!({"price":price,"ts":ts})).collect::<Vec<_>>()
        })),
        Err(_) => Json(json!({"symbol": symbol, "ticks": [], "error": "store unavailable"})),
    }
}

async fn api_alerts_check() -> Json<Value> {
    Json(json!({"triggered": []}))
}

async fn api_alert_delete() -> impl IntoResponse {
    (
        axum::http::StatusCode::NOT_FOUND,
        Json(json!({"error": "price alerts are not available in the Rust server"})),
    )
}

async fn api_news() -> Json<Value> {
    Json(json!({"articles": [], "message": "News endpoint"}))
}

async fn api_alerts(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(state.alerts_snapshot().await)
}

async fn api_metrics(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(state.metrics_snapshot())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Extension(session): Extension<auth::SessionToken>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state, session))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>, session: auth::SessionToken) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tick_tx.subscribe();
    let mut closed = state.auth.subscribe_closed();
    let auth = state.auth.clone();

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

    // Closes the socket as soon as its session is logged out or expires.
    let mut session_task = tokio::spawn(async move {
        let mut check = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            tokio::select! {
                msg = closed.recv() => match msg {
                    Ok(token) if token == session.as_str() => break,
                    Err(broadcast::error::RecvError::Closed) => break,
                    _ => {}
                },
                _ = check.tick() => {}
            }
            if !auth.session_valid(session.as_str()) {
                break;
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => {},
        _ = &mut recv_task => {},
        _ = &mut session_task => {},
    }
    send_task.abort();
    recv_task.abort();
    session_task.abort();
}

//
pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    run_server_with_optional_auth(None).await
}

/// The desktop IPC layer and HTTP compatibility adapter can share exactly one
/// password verifier/session authority. No duplicate credential cache.
pub async fn run_server_with_auth(
    shared_auth: Arc<Auth>,
) -> Result<(), Box<dyn std::error::Error>> {
    run_server_with_optional_auth(Some(shared_auth)).await
}

async fn run_server_with_optional_auth(
    preloaded_auth: Option<Arc<Auth>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig::from_env()?;
    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    // Bind before opening the database or starting any market connections.
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let (tick_tx, _) = pravda_fetchers::create_tick_channel();
    let auth = if let Some(auth) = preloaded_auth {
        auth
    } else {
        Arc::new(Auth::load(&config.auth_path, PBKDF2_ITERATIONS)?)
    };
    if !auth.is_configured() {
        if let Ok(initial) = std::env::var("PRAVDA_ADMIN_PASSWORD") {
            let worker = auth.clone();
            tokio::task::spawn_blocking(move || worker.set_password(&initial))
                .await?
                .map_err(|error| match error {
                    auth::AuthError::WeakPassword => {
                        "PRAVDA_ADMIN_PASSWORD must contain at least 15 characters"
                    }
                    auth::AuthError::Storage => {
                        "Cannot initialize authentication store; check PRAVDA_AUTH_PATH and permissions"
                    }
                    _ => "Cannot initialize PRAVDA_ADMIN_PASSWORD",
                })?;
        } else {
            println!("No local password set: open the dashboard to create one.");
        }
    }
    let mut store = PravdaStore::open(&config.database_path)?;
    // Migrate only at startup, before constructing or starting the market runtime.
    // Explicit paths win; never create a missing legacy source.
    let source = std::env::var_os("PRAVDA_LEGACY_DB_PATH")
        .or_else(|| std::env::var_os("MARKET_HISTORY_DB"))
        .or_else(|| {
            let default = std::path::PathBuf::from("market_history.sqlite3");
            default.exists().then_some(default.into_os_string())
        });
    if let Some(path) = source {
        if path.is_empty() {
            return Err("Legacy SQLite database path cannot be empty".into());
        }
        let report = store.migrate_legacy_schema(std::path::Path::new(&path))?;
        println!(
            "Legacy history migration verified: {} source rows, {} inserted, {} already imported",
            report.source_rows, report.inserted, report.already_imported
        );
    }
    let policy = runtime_policy(&config);
    let runtime = Arc::new(PravdaRuntime::new(store, tick_tx));
    let state = Arc::new(AppState {
        config,
        runtime,
        auth,
    });
    state.runtime.start_market_streams(policy).await;

    let app = Router::new()
        .route("/", get(dashboard))
        .route("/api/market", get(api_market))
        .route("/api/fx", get(api_fx))
        .route("/api/history", get(api_history))
        .route("/api/market/history", get(history::json_history))
        .route("/api/market/history.csv", get(history::csv_history))
        .route("/api/news", get(api_news))
        .route("/api/alerts", get(api_alerts))
        .route("/api/alerts/check", get(api_alerts_check))
        .route("/api/alerts/:id", axum::routing::delete(api_alert_delete))
        .route("/api/metrics", get(api_metrics))
        .route("/ws/ticks", get(ws_handler))
        .merge(auth::routes(state.auth.clone()))
        .layer(axum::middleware::from_fn_with_state(
            state.auth.clone(),
            auth::require_auth,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.config.port,
            http_security::local_only,
        ))
        .with_state(state);
    println!("Pravda Server listening on http://{addr}");
    println!("WebSocket endpoint: ws://{addr}/ws/ticks");
    axum::serve(listener, app).await?;

    Ok(())
}

// Keep the existing server-side contract tests while the production market
// loop uses the exact same transport-agnostic guard in pravda-core.
#[cfg(test)]
fn configured_spread(
    kraken: &pravda_fetchers::types::Tick,
    binance: &pravda_fetchers::types::Tick,
    now: std::time::Instant,
    config: &ServerConfig,
) -> Option<pravda_market_core::Spread> {
    pravda_core::guard_spread(kraken, binance, now, &runtime_policy(config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use pravda_fetchers::types::{Exchange, Symbol, Tick};
    use std::time::{Duration, Instant};
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
