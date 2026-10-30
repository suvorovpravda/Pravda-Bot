use crate::types::{Exchange, Symbol, Tick};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};

pub struct BinanceClient {
    url: String,
    symbols: Vec<Symbol>,
    tx: broadcast::Sender<Tick>,
}

impl BinanceClient {
    pub fn new(symbols: Vec<Symbol>, tx: broadcast::Sender<Tick>) -> Self {
        Self {
            url: "wss://stream.binance.com:9443/ws".to_string(),
            symbols,
            tx,
        }
    }

    pub async fn connect(&self) -> Result<(), Box<dyn std::error::Error>> {
        let (ws_stream, _) = connect_async(&self.url).await?;
        info!("Connected to Binance WebSocket");

        let (mut write, mut read) = ws_stream.split();

        for symbol in &self.symbols {
            let pair = match symbol {
                Symbol::BTCUSDT => "btcusdt",
                Symbol::ETHUSDT => "ethusdt",
                Symbol::SOLUSDT => "solusdt",
            };
            let subscribe_msg = serde_json::json!({
                "method": "SUBSCRIBE",
                "params": [format!("{}@ticker", pair)],
                "id": 1
            });
            write
                .send(Message::Text(subscribe_msg.to_string().into()))
                .await?;
            info!("Subscribed to {} ticker", symbol);
        }

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(tick) = self.parse_ticker_message(&text) {
                        info!("Received tick: {:?}", tick);
                        let _ = self.tx.send(tick);
                    }
                }
                Ok(Message::Ping(_)) => {}
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn parse_ticker_message(&self, text: &str) -> Result<Tick, Box<dyn std::error::Error>> {
        let value: Value = serde_json::from_str(text)?;
        if let Some(symbol_str) = value.get("s").and_then(|v| v.as_str()) {
            let symbol = match symbol_str {
                "BTCUSDT" => Symbol::BTCUSDT,
                "ETHUSDT" => Symbol::ETHUSDT,
                "SOLUSDT" => Symbol::SOLUSDT,
                _ => return Err("Unknown symbol".into()),
            };
            if let Some(price_str) = value.get("c").and_then(|v| v.as_str()) {
                if let Ok(price) = price_str.parse::<f64>() {
                    if !price.is_finite() || price <= 0.0 || !self.symbols.contains(&symbol) {
                        return Err("Invalid price or unsubscribed symbol".into());
                    }
                    let timestamp = value
                        .get("E")
                        .and_then(Value::as_i64)
                        .and_then(chrono::DateTime::from_timestamp_millis)
                        .ok_or("Missing or invalid event timestamp")?;
                    if timestamp > Utc::now() + chrono::Duration::seconds(5) {
                        return Err("Future event timestamp".into());
                    }
                    let age = (Utc::now() - timestamp).to_std().unwrap_or(Duration::ZERO);
                    let observed_at = std::time::Instant::now()
                        .checked_sub(age)
                        .ok_or("Event timestamp out of monotonic range")?;
                    return Ok(Tick {
                        symbol,
                        price,
                        timestamp,
                        exchange: Exchange::Binance,
                        observed_at,
                    });
                }
            }
        }
        Err("Invalid ticker message".into())
    }

    pub async fn connect_with_retry(&self) {
        let mut delay = Duration::from_secs(1);
        let max_delay = Duration::from_secs(60);

        loop {
            match self.connect().await {
                Ok(()) => {
                    warn!("Binance WebSocket disconnected");
                    delay = Duration::from_secs(1);
                }
                Err(e) => {
                    error!("Binance connection failed: {}", e);
                }
            }

            info!("Reconnecting in {:?}...", delay);
            sleep(delay).await;
            delay = std::cmp::min(delay * 2, max_delay);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retains_event_age_and_rejects_invalid_quotes() {
        let (tx, _) = broadcast::channel(4);
        let client = BinanceClient::new(vec![Symbol::BTCUSDT], tx);
        let epoch: i64 = (Utc::now() - chrono::Duration::seconds(20)).timestamp_millis();
        let frame = serde_json::json!({"s":"BTCUSDT","c":"100","E":epoch});
        let tick = client.parse_ticker_message(&frame.to_string()).unwrap();
        assert!(tick.observed_at.elapsed() >= Duration::from_secs(19));
        for price in ["NaN", "inf", "0", "-1", "1e999"] {
            let bad = serde_json::json!({"s":"BTCUSDT","c":price,"E":epoch});
            assert!(client.parse_ticker_message(&bad.to_string()).is_err());
        }
        assert!(client
            .parse_ticker_message(r#"{"s":"BTCUSDT","c":"100"}"#)
            .is_err());
    }
}
