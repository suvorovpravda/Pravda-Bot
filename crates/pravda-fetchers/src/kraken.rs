use crate::types::{Exchange, Symbol, Tick};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};

pub struct KrakenClient {
    url: String,
    symbols: Vec<Symbol>,
    tx: broadcast::Sender<Tick>,
}

impl KrakenClient {
    pub fn new(symbols: Vec<Symbol>, tx: broadcast::Sender<Tick>) -> Self {
        Self {
            url: "wss://ws.kraken.com".to_string(),
            symbols,
            tx,
        }
    }

    pub async fn connect(&self) -> Result<(), Box<dyn std::error::Error>> {
        let (ws_stream, _) = connect_async(&self.url).await?;
        info!("Connected to Kraken WebSocket");

        let (mut write, mut read) = ws_stream.split();

        for symbol in &self.symbols {
            let pair = match symbol {
                Symbol::BTCUSDT => "XBT/USDT",
                Symbol::ETHUSDT => "ETH/USDT",
                Symbol::SOLUSDT => "SOL/USDT",
            };
            let subscribe_msg = serde_json::json!({
                "event": "subscribe",
                "pair": [pair],
                "subscription": {"name": "ticker"}
            });
            write.send(Message::Text(subscribe_msg.to_string())).await?;
            info!("Subscribed to {} ticker", symbol);
        }

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(tick) = self.parse_ticker_message(&text) {
                        info!("Received tick: {:?}", tick);
                        // Diffuser le tick
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
        let arr: &Vec<Value> = value.as_array().ok_or("Not a ticker array")?;
        if arr.len() != 4 {
            return Err("Not a ticker frame".into());
        }
        let pair: Option<&str> = match (arr[2].as_str(), arr[3].as_str()) {
            (Some("ticker"), pair) | (pair, Some("ticker")) => pair,
            _ => return Err("Not a ticker frame".into()),
        };
        let symbol: Symbol = match pair {
            Some("XBT/USDT" | "BTC/USDT") => Symbol::BTCUSDT,
            Some("ETH/USDT") => Symbol::ETHUSDT,
            Some("SOL/USDT") => Symbol::SOLUSDT,
            _ => return Err("Unknown or non-USDT pair".into()),
        };
        let raw: &str = arr[1]
            .get("c")
            .and_then(Value::as_array)
            .and_then(|values| values.first())
            .and_then(Value::as_str)
            .ok_or("Missing close price")?;
        let price: f64 = raw.parse()?;
        if !price.is_finite() || price <= 0.0 || !self.symbols.contains(&symbol) {
            return Err("Invalid price or unsubscribed symbol".into());
        }
        // Kraken v1 ticker has no event timestamp: this records reception only.
        Ok(Tick {
            symbol,
            price,
            timestamp: Utc::now(),
            exchange: Exchange::Kraken,
            observed_at: std::time::Instant::now(),
        })
    }

    pub async fn connect_with_retry(&self) {
        let mut delay = Duration::from_secs(1);
        let max_delay = Duration::from_secs(60);

        loop {
            match self.connect().await {
                Ok(()) => {
                    warn!("Kraken WebSocket disconnected");
                    delay = Duration::from_secs(1);
                }
                Err(e) => {
                    error!("Kraken connection failed: {}", e);
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
    fn parses_real_v1_layout_and_rejects_usd_and_nonfinite() {
        let (tx, _) = broadcast::channel(4);
        let client = KrakenClient::new(vec![Symbol::BTCUSDT], tx);
        let tick = client
            .parse_ticker_message(r#"[42,{"c":["100.5","1"]},"ticker","XBT/USDT"]"#)
            .unwrap();
        assert_eq!(tick.price, 100.5);
        for frame in [
            r#"[42,{"c":["100","1"]},"ticker","XBT/USD"]"#,
            r#"[42,{"c":["NaN","1"]},"ticker","XBT/USDT"]"#,
            r#"[42,{"c":["0","1"]},"ticker","XBT/USDT"]"#,
            r#"{"event":"heartbeat"}"#,
        ] {
            assert!(client.parse_ticker_message(frame).is_err());
        }
    }
}
