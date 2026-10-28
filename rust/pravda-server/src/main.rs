use axum::{
    extract::State,
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response, Json},
    routing::get,
    Router,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

/// Mock data manager for API endpoints (replace with real backend later)
struct DataManager {
    markets: RwLock<Option<Value>>,
    fx: RwLock<Option<Value>>,
    history: RwLock<Option<Value>>,
    news: RwLock<Option<Value>>,
    alerts: RwLock<Option<Value>>,
}

impl DataManager {
    fn new() -> Self {
        let data = json!({
            "BTC/USDT": {"exchange": "binance", "price": 79050.50, "volume_24h": 15000000000.0},
            "ETH/USDT": {"exchange": "kraken", "price": 4025.25, "volume_24h": 8000000000.0},
        });
        Self {
            markets: RwLock::new(Some(json!({"status": "ok", "data": data}))),
            fx: RwLock::new(Some(json!({
                "EUR/USD": 1.0875,
                "EUR/CHF": 0.9625,
                "USD/JPY": 149.50
            }))),
            history: RwLock::new(Some(json!({
                "BTC/USDT": {
                    "timestamps": ["2024-01-01T00:00:00Z"],
                    "prices": [78000.0]
                }
            }))),
            news: RwLock::new(Some(json!({
                "items": [
                    {"title": "BTC rally continues", "source": "Coinglass"},
                    {"title": "EU gas prices stabilize", "source": "ENTSO-E"}
                ]
            }))),
            alerts: RwLock::new(Some(json!({"status": "ok", "alerts": []}))),
        }
    }

    fn market(&self) -> Json<Value> {
        Json(self.markets.read().unwrap().as_ref().unwrap().clone())
    }

    fn fx(&self) -> Json<Value> {
        Json(self.fx.read().unwrap().as_ref().unwrap().clone())
    }

    fn history(&self) -> Json<Value> {
        Json(self.history.read().unwrap().as_ref().unwrap().clone())
    }

    fn news(&self) -> Json<Value> {
        Json(self.news.read().unwrap().as_ref().unwrap().clone())
    }

    fn alerts(&self) -> Json<Value> {
        Json(self.alerts.read().unwrap().as_ref().unwrap().clone())
    }
}

/// Dashboard HTML with security headers (CSP, X-Frame-Options)
async fn dashboard_html() -> impl IntoResponse {
    let html = include_str!("../static/dashboard.html");
    let mut response = Response::new(html.to_string());
    let headers = response.headers_mut();
    
    // Strict CSP: same-origin only, allow inline scripts for dashboard functionality
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:;"),
    );
    headers.insert(
        "X-Frame-Options",
        HeaderValue::from_static("DENY"),
    );
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

/// API endpoint: market data (polling for dashboard)
async fn api_market() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "data": {
            "BTC/USDT": {"exchange": "binance", "price": 79050.50, "volume_24h": 15000000000.0},
            "ETH/USDT": {"exchange": "kraken", "price": 4025.25, "volume_24h": 8000000000.0},
        }
    }))
}

/// API endpoint: FX reference rates
async fn api_fx() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "data": {
            "EUR/USD": 1.0875,
            "EUR/CHF": 0.9625,
            "USD/JPY": 149.50
        }
    }))
}

/// API endpoint: historical prices (for charts)
async fn api_history() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "data": {
            "BTC/USDT": {
                "timestamps": ["2024-01-01T00:00:00Z"],
                "prices": [78000.0, 79050.50]
            },
            "ETH/USDT": {
                "timestamps": ["2024-01-01T00:00:00Z"],
                "prices": [3800.0, 4025.25]
            }
        }
    }))
}

/// API endpoint: news and alerts feed
async fn api_news() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "data": {
            "items": [
                {"title": "BTC rally continues", "source": "Coinglass", "time": "2024-01-02T10:00:00Z"},
                {"title": "EU gas prices stabilize at TTF", "source": "ENTSO-E", "time": "2024-01-02T08:30:00Z"}
            ]
        }
    }))
}

/// API endpoint: price alerts management (paper mode)
async fn api_alerts() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "data": {
            "alerts": [],
            "settings": {"mode": "paper", "max_hold_seconds": 3600}
        }
    }))
}

/// API endpoint: system metrics (mock)
async fn api_metrics() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "data": {
            "uptime_seconds": 3600.0,
            "memory_usage_mb": 128.5,
            "cpu_percent": 12.3,
            "db_size_bytes": 1048576
        }
    }))
}

#[tokio::main]
async fn main() {
    let data = Arc::new(DataManager::new());
    
    let app = Router::new()
        // Dashboard routes
        .route("/", get(dashboard_html))
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
