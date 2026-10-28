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
                Symbol::BTCUSDT => "XBTUSD",
                Symbol::ETHUSDT => "ETHUSD",
                Symbol::SOLUSDT => "SOLUSD",
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
        if let Some(arr) = value.as_array() {
            if let Some(channel_data) = arr.get(1).and_then(|v| v.as_object()) {
                if let Some(pair) = channel_data.get("pair").and_then(|v| v.as_str()) {
                    let symbol = match pair {
                        "XBTUSD" => Symbol::BTCUSDT,
                        "ETHUSD" => Symbol::ETHUSDT,
                        "SOLUSD" => Symbol::SOLUSDT,
                        _ => return Err("Unknown pair".into()),
                    };
                    if let Some(ticker) = channel_data.get("ticker") {
                        if let Some(last) = ticker
                            .get("c")
                            .and_then(|v| v.as_array())
                            .and_then(|a| a.first())
                            .and_then(|v| v.as_str())
                        {
                            if let Ok(price) = last.parse::<f64>() {
                                return Ok(Tick {
                                    symbol,
                                    price,
                                    timestamp: Utc::now(),
                                    exchange: Exchange::Kraken,
                                });
                            }
                        }
                    }
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
