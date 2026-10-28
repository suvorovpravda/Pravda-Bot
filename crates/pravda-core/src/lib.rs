pub mod error;
pub mod types;

pub use error::PravdaError;
pub use types::{EnergyPrice, Exchange, MiningIndex, Spread, Symbol, Tick};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn types_are_constructible() {
        let _ = Symbol::BtcUsdt;
        let _ = Exchange::Kraken;
    }
}
