pub mod binance;
pub mod energy;
pub mod kraken;
pub mod public_feeds;
pub mod revolut_x;
pub mod revolut_x_lifecycle;
pub mod revolut_x_orders;
pub mod revolut_x_reconcile;
pub mod revolut_x_private;
pub mod types;

pub use binance::BinanceClient;
pub use kraken::KrakenClient;
pub use types::{Exchange, Spread, Symbol, Tick};

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
