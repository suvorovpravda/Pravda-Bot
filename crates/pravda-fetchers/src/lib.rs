pub mod types;
pub mod kraken;
pub mod binance;
pub mod health;

pub use types::{Symbol, Exchange, Tick, Spread};
pub use kraken::KrakenClient;
pub use binance::BinanceClient;
pub use health::{MarketHealth, HealthStatus, assess_market_data, calculate_spread};
