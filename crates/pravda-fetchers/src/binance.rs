use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use serde_json::Value;
use crate::types::{Tick, Symbol, Exchange};
use chrono::Utc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error};

pub struct BinanceClient {
    url: String,
    symbols: Vec<Symbol>,
}

impl BinanceClient {
    pub fn new(symbols: Vec<Symbol>) -> Self {
        Self {
            url: "wss://stream.binance.com:9443/ws".to_string(),
            symbols,
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
            write.send(Message::Text(subscribe_msg.to_string())).await?;
            info!("Subscribed to {} ticker", symbol);
        }

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(tick) = self.parse_ticker_message(&text) {
                        info!("Received tick: {:?}", tick);
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
                    return Ok(Tick {
                        symbol,
                        price,
                        timestamp: Utc::now(),
                        exchange: Exchange::Binance,
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
