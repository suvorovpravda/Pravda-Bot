pub mod error;
pub mod types;

pub use error::PravdaError;
pub use types::{Symbol, Exchange, Tick, Spread, EnergyPrice, MiningIndex};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn types_are_constructible() {
        let _ = Symbol::BtcUsdt;
        let _ = Exchange::Kraken;
    }
}
