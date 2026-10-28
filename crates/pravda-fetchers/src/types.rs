use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Symbol {
    BTCUSDT,
    ETHUSDT,
    SOLUSDT,
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Symbol::BTCUSDT => write!(f, "BTC/USDT"),
            Symbol::ETHUSDT => write!(f, "ETH/USDT"),
            Symbol::SOLUSDT => write!(f, "SOL/USDT"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Exchange {
    Kraken,
    Binance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tick {
    pub symbol: Symbol,
    pub price: f64,
    pub timestamp: DateTime<Utc>,
    pub exchange: Exchange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spread {
    pub symbol: Symbol,
    pub gross_spread: f64,
    pub net_spread: f64,
    pub timestamp: DateTime<Utc>,
}
