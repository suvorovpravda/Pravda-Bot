use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Symbol {
    BtcUsdt,
    EthUsdt,
    SolUsdt,
}

impl Symbol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Symbol::BtcUsdt => "BTC/USDT",
            Symbol::EthUsdt => "ETH/USDT",
            Symbol::SolUsdt => "SOL/USDT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Exchange {
    Kraken,
    Binance,
}

impl Exchange {
    pub fn as_str(&self) -> &'static str {
        match self {
            Exchange::Kraken => "Kraken",
            Exchange::Binance => "Binance",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tick {
    pub timestamp: DateTime<Utc>,
    pub symbol: Symbol,
    pub exchange: Exchange,
    pub price: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Spread {
    pub gross_fraction: f64,
    pub net_fraction: f64,
}

impl Spread {
    pub fn gross_percent(&self) -> f64 {
        self.gross_fraction * 100.0
    }
    pub fn net_percent(&self) -> f64 {
        self.net_fraction * 100.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyPrice {
    pub zone: String,
    pub price_eur_mwh: f64,
    pub timestamp: DateTime<Utc>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningIndex {
    pub timestamp: DateTime<Utc>,
    pub btc_price_usd: f64,
    pub hashprice_usd_th: f64,
    pub electricity_price_eur_mwh: f64,
    pub profitability_eur_per_th_day: f64,
}
