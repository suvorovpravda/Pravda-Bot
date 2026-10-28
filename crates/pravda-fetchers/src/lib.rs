pub mod types;
pub mod kraken;
pub mod binance;
pub mod health;

pub use types::{Symbol, Exchange, Tick, Spread};
pub use kraken::KrakenClient;
pub use binance::BinanceClient;
pub use health::{MarketHealth, HealthStatus, assess_market_data, calculate_spread};

use tokio::sync::broadcast;

/// Taille du canal broadcast pour les ticks
const CHANNEL_CAPACITY: usize = 1024;

/// Crée un canal broadcast pour les ticks
pub fn create_tick_channel() -> (broadcast::Sender<Tick>, broadcast::Receiver<Tick>) {
    broadcast::channel(CHANNEL_CAPACITY)
}

/// Lance les deux clients WebSocket en parallèle avec reconnexion
pub async fn run_fetchers(
    tx: broadcast::Sender<Tick>,
    symbols: Vec<types::Symbol>,
) -> Result<(), Box<dyn std::error::Error>> {
    let kraken = kraken::KrakenClient::new(symbols.clone(), tx.clone());
    let binance = binance::BinanceClient::new(symbols.clone(), tx.clone());

    tokio::select! {
        _ = kraken.connect_with_retry() => {},
        _ = binance.connect_with_retry() => {},
    }

    Ok(())
}
