//! Bounded history endpoints shared by JSON and CSV consumers.
use crate::AppState;
use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Duration, Utc};
use pravda_store::HistoryTick;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub enum HistorySymbol {
    #[default]
    #[serde(rename = "BTC/USDT")]
    Btc,
    #[serde(rename = "ETH/USDT")]
    Eth,
    #[serde(rename = "SOL/USDT")]
    Sol,
}
impl HistorySymbol {
    fn name(self) -> &'static str {
        match self {
            Self::Btc => "BTC/USDT",
            Self::Eth => "ETH/USDT",
            Self::Sol => "SOL/USDT",
        }
    }
}
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub enum HistoryExchange {
    #[default]
    Binance,
    Kraken,
}
impl HistoryExchange {
    fn name(self) -> &'static str {
        match self {
            Self::Binance => "Binance",
            Self::Kraken => "Kraken",
        }
    }
}
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HistoryPeriod {
    #[default]
    Day,
    Week,
    Month,
}
impl HistoryPeriod {
    fn duration(self) -> Duration {
        Duration::days(match self {
            Self::Day => 1,
            Self::Week => 7,
            Self::Month => 30,
        })
    }
}
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct HistoryQuery {
    symbol: HistorySymbol,
    exchange: HistoryExchange,
    period: HistoryPeriod,
}
#[derive(Debug, Serialize)]
struct Observation {
    price: f64,
    timestamp: String,
    ticks: u32,
}
#[derive(Debug, Serialize)]
struct HistoryResponse {
    symbol: HistorySymbol,
    exchange: HistoryExchange,
    period: HistoryPeriod,
    observations: Vec<Observation>,
    truncated: bool,
}

async fn read_history(
    state: Arc<AppState>,
    query: HistoryQuery,
) -> Result<(Vec<HistoryTick>, bool), StatusCode> {
    let store = state.store.clone();
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
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let truncated = rows.len() > 10_000;
    if truncated {
        rows.remove(0);
    }
    // Reject corrupt persisted values rather than exporting invalid observations.
    if rows.iter().any(|r| {
        !r.price.is_finite()
            || r.price <= 0.0
            || chrono::DateTime::parse_from_rfc3339(&r.timestamp).is_err()
    }) {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    Ok((rows, truncated))
}

pub(crate) async fn json_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> Response {
    match read_history(state, query).await {
        Ok((rows, truncated)) => (
            [(header::CACHE_CONTROL, "no-store")],
            Json(HistoryResponse {
                symbol: query.symbol,
                exchange: query.exchange,
                period: query.period,
                observations: rows
                    .into_iter()
                    .map(|r| Observation {
                        price: r.price,
                        timestamp: r.timestamp,
                        ticks: 1,
                    })
                    .collect(),
                truncated,
            }),
        )
            .into_response(),
        Err(status) => (
            status,
            Json(serde_json::json!({"error":"history unavailable"})),
        )
            .into_response(),
    }
}

pub(crate) async fn csv_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> Response {
    match read_history(state, query).await {
        Ok((rows, truncated)) => {
            // All fields are enum-controlled, validated timestamps, or positive numbers.
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
            (
                [
                    (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                    (
                        header::CONTENT_DISPOSITION,
                        "attachment; filename=market-history.csv",
                    ),
                    (header::CACHE_CONTROL, "no-store"),
                    (
                        axum::http::HeaderName::from_static("x-history-truncated"),
                        if truncated { "true" } else { "false" },
                    ),
                ],
                csv,
            )
                .into_response()
        }
        Err(status) => (
            status,
            Json(serde_json::json!({"error":"history unavailable"})),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::Request,
        routing::get,
        Router,
    };
    use pravda_engine::{PaperMode, PaperSimulator};
    use pravda_store::PravdaStore;
    use std::{collections::HashMap, time::Instant};
    use tokio::sync::{broadcast, Mutex, RwLock};
    use tower::ServiceExt;

    fn test_state() -> Arc<AppState> {
        let store = PravdaStore::open(":memory:").unwrap();
        let now = Utc::now();
        store
            .insert_tick("ETH/USDT", 200.0, "Kraken", now - Duration::minutes(2))
            .unwrap();
        store
            .insert_tick("ETH/USDT", 201.0, "Kraken", now - Duration::minutes(1))
            .unwrap();
        store
            .insert_tick("ETH/USDT", 202.0, "Binance", now - Duration::minutes(1))
            .unwrap();
        store
            .insert_tick("ETH/USDT", 190.0, "Kraken", now - Duration::days(2))
            .unwrap();
        let (tick_tx, _) = broadcast::channel(8);
        Arc::new(AppState {
            config: crate::config::ServerConfig::default(),
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
            store: Arc::new(Mutex::new(store)),
        })
    }
    #[tokio::test]
    async fn alerts_handler_releases_alerts_before_waiting_for_simulator() {
        let state = test_state();
        let simulator = state.paper_sim.write().await;
        let response = crate::api_alerts(State(state.clone()));
        tokio::pin!(response);
        std::future::poll_fn(|context| {
            assert!(std::future::Future::poll(response.as_mut(), context).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        assert!(
            state.alerts.try_write().is_ok(),
            "opposite lock order can deadlock the writer"
        );
        drop(simulator);
        let _ = response.await;
    }

    fn app() -> Router {
        Router::new()
            .route("/api/market/history", get(json_history))
            .route("/api/market/history.csv", get(csv_history))
            .with_state(test_state())
    }
    #[tokio::test]
    async fn configured_port_reaches_dashboard_socket_and_csp() {
        let mut state = test_state();
        Arc::get_mut(&mut state).unwrap().config.port = 8081;
        let response = crate::dashboard(State(state)).await.into_response();
        assert!(response.headers()[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap()
            .contains("ws://127.0.0.1:8081"));
        let bytes = to_bytes(response.into_body(), 2_000_000).await.unwrap();
        let html = std::str::from_utf8(&bytes).unwrap();
        assert!(html.contains("ws://127.0.0.1:8081/ws/ticks"));
        assert!(!html.contains("ws://127.0.0.1:8080/ws/ticks"));
    }
    #[tokio::test]
    async fn cap_keeps_newest_ticks_and_reports_truncation() {
        let state = test_state();
        {
            let store = state.store.lock().await;
            let now = Utc::now() - Duration::seconds(2);
            for index in 0_u32..10_002 {
                store
                    .insert_tick("BTC/USDT", f64::from(index + 1), "Binance", now)
                    .unwrap();
            }
        }
        let (rows, truncated) = read_history(state, HistoryQuery::default()).await.unwrap();
        assert!(truncated);
        assert_eq!(rows.len(), 10_000);
        assert_eq!(rows.first().unwrap().price, 3.0);
        assert_eq!(rows.last().unwrap().price, 10_002.0);
    }
    #[tokio::test]
    async fn empty_history_is_success_not_a_fake_observation() {
        let (rows, truncated) = read_history(test_state(), HistoryQuery::default())
            .await
            .unwrap();
        assert!(rows.is_empty());
        assert!(!truncated);
    }
    #[tokio::test]
    async fn json_matches_dashboard_contract_and_filters_exchange_period() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/api/market/history?symbol=ETH%2FUSDT&exchange=Kraken&period=day")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(payload["symbol"], "ETH/USDT");
        assert_eq!(payload["observations"].as_array().unwrap().len(), 2);
        assert_eq!(payload["observations"][0]["price"], 200.0);
        assert_eq!(payload["observations"][1]["price"], 201.0);
        assert_eq!(payload["observations"][0]["ticks"], 1);
        assert_eq!(payload["truncated"], false);
    }
    #[tokio::test]
    async fn csv_is_downloadable_and_week_includes_older_ticks() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/api/market/history.csv?symbol=ETH%2FUSDT&exchange=Kraken&period=week")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/csv; charset=utf-8"
        );
        let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert_eq!(text.lines().count(), 4);
        assert!(text
            .lines()
            .nth(1)
            .unwrap()
            .ends_with(",ETH/USDT,Kraken,190"));
        assert!(!text.contains("Binance"));
    }
    #[tokio::test]
    async fn invalid_filters_are_rejected_not_silently_ignored() {
        for query in [
            "symbol=INVALID",
            "exchange=Other",
            "period=forever",
            "limit=999999999",
            "symbol=%27%3BDROP%20TABLE%20ticks",
        ] {
            let response = app()
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/market/history?{query}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{query}");
        }
    }
}
