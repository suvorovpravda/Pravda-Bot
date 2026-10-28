use crate::types::{Spread, Symbol, Tick};
use chrono::{Duration, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Waiting,
    Stale,
    Desynchronized,
    Ok,
}

#[derive(Debug, Clone)]
pub struct MarketHealth {
    pub status: HealthStatus,
    pub kraken_age: Option<Duration>,
    pub binance_age: Option<Duration>,
    pub exchange_skew: Option<Duration>,
}

pub fn assess_market_data(
    kraken_tick: Option<&Tick>,
    binance_tick: Option<&Tick>,
    stale_after: Duration,
    max_skew: Duration,
) -> MarketHealth {
    let now = Utc::now();
    let kraken_age = kraken_tick.map(|t| now.signed_duration_since(t.timestamp));
    let binance_age = binance_tick.map(|t| now.signed_duration_since(t.timestamp));

    if kraken_tick.is_none() || binance_tick.is_none() {
        return MarketHealth {
            status: HealthStatus::Waiting,
            kraken_age: None,
            binance_age: None,
            exchange_skew: None,
        };
    }

    let kraken_age_dur = kraken_age.unwrap();
    let binance_age_dur = binance_age.unwrap();
    let skew = (kraken_tick.unwrap().timestamp - binance_tick.unwrap().timestamp).abs();

    let status = if kraken_age_dur > stale_after || binance_age_dur > stale_after {
        HealthStatus::Stale
    } else if skew > max_skew {
        HealthStatus::Desynchronized
    } else {
        HealthStatus::Ok
    };

    MarketHealth {
        status,
        kraken_age: Some(kraken_age_dur),
        binance_age: Some(binance_age_dur),
        exchange_skew: Some(skew),
    }
}

pub fn calculate_spread(kraken_price: f64, binance_price: f64, fee_rate: f64) -> Spread {
    let gross = (kraken_price - binance_price).abs() / kraken_price.min(binance_price);
    let net = gross - fee_rate;
    Spread {
        symbol: Symbol::BTCUSDT, // placeholder
        gross_spread: gross,
        net_spread: net,
        timestamp: Utc::now(),
    }
}
