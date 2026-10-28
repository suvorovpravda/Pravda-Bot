use axum::{
    http::HeaderValue,
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use serde_json::{json, Value};
use std::net::SocketAddr;

/// Dashboard HTML with strict CSP and security headers
async fn dashboard() -> impl IntoResponse {
    let html = include_str!("../static/dashboard.html");
    let mut response = Response::new(html.to_string());
    let headers = response.headers_mut();

    // Strict CSP: same-origin, allow inline scripts for existing dashboard functionality
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:;"),
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

/// API: market data (matches frontend renderMarket expectations)
async fn api_market() -> Json<Value> {
    Json(json!({
        "symbols": {
            "BTC/USDT": {
                "binance": {"price": 79050.50, "volume_24h": 15000000000.0},
                "kraken": {"price": 79045.20, "volume_24h": 8000000000.0}
            },
            "ETH/USDT": {
                "binance": {"price": 4025.25, "volume_24h": 8000000000.0},
                "kraken": {"price": 4023.10, "volume_24h": 4000000000.0}
            },
            "SOL/USDT": {
                "binance": {"price": 185.50, "volume_24h": 2000000000.0},
                "kraken": {"price": 185.45, "volume_24h": 1000000000.0}
            }
        },
        "timestamp": "2026-10-29T14:00:00Z"
    }))
}

/// API: FX rates (matches frontend renderFx expectations)
async fn api_fx() -> Json<Value> {
    Json(json!({
        "rates": {
            "EUR": {"USD": 1.085, "CHF": 0.945},
            "USD": {"EUR": 0.922, "CHF": 0.871}
        },
        "source": "Frankfurter (ECB)",
        "timestamp": "2026-10-29"
    }))
}

/// API: historical prices (placeholder for future SQLite integration)
async fn api_history() -> Json<Value> {
    Json(json!({
        "ticks": [],
        "message": "History endpoint - to be connected to SQLite WAL persistence"
    }))
}

/// API: news and alerts feed (placeholder for GDELT/BBC integration)
async fn api_news() -> Json<Value> {
    Json(json!({
        "articles": [],
        "message": "News endpoint - to be connected to GDELT/BBC RSS"
    }))
}

/// API: alerts management (paper mode only, no real execution)
async fn api_alerts() -> Json<Value> {
    Json(json!({
        "alerts": [],
        "settings": {"mode": "paper", "max_hold_seconds": 3600}
    }))
}

/// API: system metrics (mock for UI visualization)
async fn api_metrics() -> Json<Value> {
    Json(json!({
        "uptime_seconds": 3600.0,
        "memory_usage_mb": 128.5,
        "cpu_percent": 12.3,
        "db_size_bytes": 1048576
    }))
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        // Dashboard routes
        .route("/", get(dashboard))
        .route("/api/market", get(api_market))
        .route("/api/fx", get(api_fx))
        .route("/api/history", get(api_history))
        .route("/api/news", get(api_news))
        .route("/api/alerts", get(api_alerts))
        .route("/api/metrics", get(api_metrics));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("🚀 Pravda Server listening on http://127.0.0.1:8080");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
