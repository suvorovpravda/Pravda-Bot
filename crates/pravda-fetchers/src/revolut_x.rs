//! Revolut X public order-book snapshots only.
//!
//! Official endpoint: https://revx.revolut.com/api/2.0/public/order-book/{symbol}
//! No credentials, balances, order submission or signing are supported here.
//! Public limit: 1 request/second; callers must not treat snapshots as live fills.

use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::Mutex;

const MAX_RESPONSE_BYTES: usize = 512 * 1024;
const MIN_REQUEST_INTERVAL: Duration = Duration::from_secs(1);
const BASE_URL: &str = "https://revx.revolut.com/api/2.0/public/order-book/";

/// A read-only contract, deliberately separate from any order execution API.
pub trait PublicMarketDataAdapter {
    fn venue(&self) -> &'static str;
    fn public_order_book_url(&self, symbol: &str) -> Result<Url, PublicMarketError>;
}

#[derive(Debug, Error)]
pub enum PublicMarketError {
    #[error("unsupported or unsafe symbol")]
    InvalidSymbol,
    #[error("invalid price or quantity in market response")]
    InvalidDecimal,
    #[error("invalid exchange response")]
    InvalidResponse,
    #[error("exchange rate limit exceeded")]
    RateLimited,
    #[error("unexpected exchange HTTP status: {0}")]
    HttpStatus(u16),
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("response JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct OrderLevel {
    pub price: String,
    pub quantity: String,
    pub count: u64,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct OrderBookData {
    pub asks: Vec<OrderLevel>,
    pub bids: Vec<OrderLevel>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct OrderBookMetadata {
    pub timestamp: i64,
    pub region: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PublicOrderBook {
    pub data: OrderBookData,
    pub metadata: OrderBookMetadata,
}

impl PublicOrderBook {
    fn validate(&self) -> Result<(), PublicMarketError> {
        if self.metadata.timestamp <= 0 {
            return Err(PublicMarketError::InvalidResponse);
        }
        for level in self.data.asks.iter().chain(self.data.bids.iter()) {
            if !positive_decimal(&level.price) || !positive_decimal(&level.quantity) {
                return Err(PublicMarketError::InvalidDecimal);
            }
        }
        Ok(())
    }
}

/// Validate decimal text without converting a monetary quantity to f64.
fn positive_decimal(input: &str) -> bool {
    if input.is_empty() || input.len() > 50 {
        return false;
    }
    let mut dot_seen = false;
    let mut any_digit = false;
    let mut nonzero_digit = false;
    for ch in input.bytes() {
        match ch {
            b'0'..=b'9' => {
                any_digit = true;
                nonzero_digit |= ch != b'0';
            }
            b'.' if !dot_seen => dot_seen = true,
            _ => return false,
        }
    }
    any_digit && nonzero_digit && !input.starts_with('.') && !input.ends_with('.')
}

/// Safe opt-in public client; never installs a trading permission or API key.
#[derive(Clone)]
pub struct RevolutXPublicClient {
    client: Client,
    throttle: Arc<Mutex<Option<Instant>>>,
}

impl RevolutXPublicClient {
    pub fn new() -> Result<Self, PublicMarketError> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(8))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            throttle: Arc::new(Mutex::new(None)),
        })
    }

    /// Request one public snapshot, not a stream or executable order quote.
    pub async fn order_book(&self, symbol: &str) -> Result<PublicOrderBook, PublicMarketError> {
        let url = self.public_order_book_url(symbol)?;
        // Shared by clones. Serializes calls to respect the public 1 req/s limit.
        let mut last = self.throttle.lock().await;
        if let Some(previous) = *last {
            let remaining = MIN_REQUEST_INTERVAL.saturating_sub(previous.elapsed());
            if !remaining.is_zero() {
                tokio::time::sleep(remaining).await;
            }
        }
        *last = Some(Instant::now());
        drop(last);

        let mut response = self.client.get(url).send().await?;
        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            return Err(PublicMarketError::RateLimited);
        }
        if !response.status().is_success() {
            return Err(PublicMarketError::HttpStatus(response.status().as_u16()));
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return Err(PublicMarketError::InvalidResponse);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(PublicMarketError::InvalidResponse);
            }
            bytes.extend_from_slice(&chunk);
        }
        let book: PublicOrderBook = serde_json::from_slice(&bytes)?;
        book.validate()?;
        Ok(book)
    }
}

impl PublicMarketDataAdapter for RevolutXPublicClient {
    fn venue(&self) -> &'static str {
        "Revolut X"
    }

    fn public_order_book_url(&self, symbol: &str) -> Result<Url, PublicMarketError> {
        // Explicit pairs until the instrument directory is validated separately.
        if !matches!(symbol, "BTC-USD" | "ETH-USD" | "SOL-USD") {
            return Err(PublicMarketError::InvalidSymbol);
        }
        let mut url = Url::parse(BASE_URL).map_err(|_| PublicMarketError::InvalidResponse)?;
        url.path_segments_mut()
            .map_err(|_| PublicMarketError::InvalidResponse)?
            .pop_if_empty()
            .push(symbol);
        Ok(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_endpoint_is_v2_and_does_not_embed_credentials() {
        let client = RevolutXPublicClient::new().unwrap();
        assert_eq!(client.venue(), "Revolut X");
        assert_eq!(
            client.public_order_book_url("BTC-USD").unwrap().as_str(),
            "https://revx.revolut.com/api/2.0/public/order-book/BTC-USD"
        );
        for value in ["../balances", "BTC/USD", "BTC-USD?limit=999", "ETH-USDT"] {
            assert!(client.public_order_book_url(value).is_err());
        }
    }

    #[test]
    fn parses_public_order_book_with_exact_decimal_strings() {
        let raw = r#"{"data":{"asks":[{"price":"4005.00","quantity":"1.7000","count":3}],"bids":[{"price":"4000.00","quantity":"0.25","count":1}]},"metadata":{"region":"UK","timestamp":1785313433816}}"#;
        let parsed: PublicOrderBook = serde_json::from_str(raw).unwrap();
        parsed.validate().unwrap();
        assert_eq!(parsed.data.asks[0].price, "4005.00");
        assert_eq!(parsed.data.bids[0].quantity, "0.25");
    }

    #[test]
    fn rejects_invalid_decimal_and_metadata() {
        for input in ["0", "-1", "NaN", "Infinity", "1e9", "1.2.3", ".5", "2."] {
            assert!(!positive_decimal(input), "{input}");
        }
        assert!(positive_decimal("0.00001"));
        let raw = r#"{"data":{"asks":[{"price":"NaN","quantity":"1.0","count":1}],"bids":[]},"metadata":{"region":"EEA","timestamp":42}}"#;
        let parsed: PublicOrderBook = serde_json::from_str(raw).unwrap();
        assert!(parsed.validate().is_err());
    }
}
