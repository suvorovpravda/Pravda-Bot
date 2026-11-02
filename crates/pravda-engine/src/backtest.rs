//! Deterministic, offline paper-simulation replay. No HTTP, IPC, exchange
//! credentials, executable order adapter, or dependence on wall-clock time.
//! This replays externally supplied paired quotes; it does NOT model slippage,
//! liquidity, fill probability, latency or survivorship bias.
use crate::{PaperMode, PaperSimulator};
use chrono::{DateTime, Utc};

const MAX_REPLAY_ROWS: usize = 1_000_000;

#[derive(Debug, Clone)]
pub struct ReplayTick {
    pub at: DateTime<Utc>,
    pub symbol: String,
    pub kraken_price: f64,
    pub binance_price: f64,
    /// Precomputed using data available at the same instant or earlier.
    pub opportunity_confirmed: bool,
    /// Refusal must prevent entries even when the opportunity is confirmed.
    pub execution_approved: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ReplayConfig {
    pub starting_balance_usdt: f64,
    pub fixed_position_usdt: f64,
    pub estimated_fees_percent: f64,
    pub exit_spread_percent: f64,
    pub max_hold_seconds: f64,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            starting_balance_usdt: 1_000.0,
            fixed_position_usdt: 100.0,
            estimated_fees_percent: 0.36,
            exit_spread_percent: 0.05,
            max_hold_seconds: 3_600.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayError {
    InvalidConfiguration,
    EmptyInput,
    TooManyRows,
    OutOfOrder,
    InvalidRow,
    InvalidResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplayResult {
    pub rows_processed: usize,
    pub opened: u64,
    pub closed: u64,
    pub still_open: usize,
    pub cash_usdt: f64,
    /// Book value only: excludes unrealized profit and possible liquidation fees.
    pub book_equity_usdt: f64,
    pub realized_pnl_usdt: f64,
    pub first_timestamp: DateTime<Utc>,
    pub last_timestamp: DateTime<Utc>,
}

pub fn replay(ticks: &[ReplayTick], config: ReplayConfig) -> Result<ReplayResult, ReplayError> {
    if ticks.is_empty() {
        return Err(ReplayError::EmptyInput);
    }
    if ticks.len() > MAX_REPLAY_ROWS {
        return Err(ReplayError::TooManyRows);
    }
    if !config.starting_balance_usdt.is_finite()
        || config.starting_balance_usdt <= 0.0
        || !config.fixed_position_usdt.is_finite()
        || config.fixed_position_usdt <= 0.0
        || config.fixed_position_usdt > config.starting_balance_usdt
        || !config.estimated_fees_percent.is_finite()
        || !(0.0..=100.0).contains(&config.estimated_fees_percent)
        || !config.exit_spread_percent.is_finite()
        || config.exit_spread_percent < 0.0
        || !config.max_hold_seconds.is_finite()
        || config.max_hold_seconds <= 0.0
    {
        return Err(ReplayError::InvalidConfiguration);
    }

    let mut sim = PaperSimulator::new(
        PaperMode::Fixed,
        config.starting_balance_usdt,
        0.0,
        config.fixed_position_usdt,
        config.exit_spread_percent,
        config.max_hold_seconds,
        config.estimated_fees_percent,
    );
    let mut opened = 0;
    let mut closed = 0;
    let mut previous = None;
    for tick in ticks {
        if previous.is_some_and(|time| tick.at <= time) {
            return Err(ReplayError::OutOfOrder);
        }
        previous = Some(tick.at);
        if tick.symbol.is_empty()
            || tick.symbol.len() > 32
            || !tick.symbol.bytes().all(|ch| ch.is_ascii_alphanumeric() || ch == b'/')
            || !tick.kraken_price.is_finite()
            || !tick.binance_price.is_finite()
            || tick.kraken_price <= 0.0
            || tick.binance_price <= 0.0
        {
            return Err(ReplayError::InvalidRow);
        }
        match sim.update_with_execution_approval(
            &tick.symbol,
            tick.kraken_price,
            tick.binance_price,
            tick.opportunity_confirmed,
            tick.execution_approved,
            tick.at,
        ).as_deref() {
            Some("opened") => opened += 1,
            Some("closed") => closed += 1,
            _ => {}
        }
        if !sim.cash_usdt.is_finite() || !sim.realized_pnl_usdt.is_finite() {
            return Err(ReplayError::InvalidResult);
        }
    }
    let book_equity_usdt = sim.equity_usdt();
    if !book_equity_usdt.is_finite() {
        return Err(ReplayError::InvalidResult);
    }
    Ok(ReplayResult {
        rows_processed: ticks.len(),
        opened,
        closed,
        still_open: sim.positions.len(),
        cash_usdt: sim.cash_usdt,
        book_equity_usdt,
        realized_pnl_usdt: sim.realized_pnl_usdt,
        first_timestamp: ticks[0].at,
        last_timestamp: ticks[ticks.len()-1].at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tick(offset: i64, k: f64, b: f64, approved: bool) -> ReplayTick {
        ReplayTick {
            at: DateTime::from_timestamp(1_700_000_000 + offset, 0).unwrap(),
            symbol: "BTC/USDT".to_string(),
            kraken_price: k,
            binance_price: b,
            opportunity_confirmed: true,
            execution_approved: approved,
        }
    }

    #[test]
    fn identical_replays_have_identical_cash_and_pnl() {
        let data = [tick(0, 101.0, 100.0, true), tick(10, 99.0, 100.0, false)];
        let first = replay(&data, ReplayConfig::default()).unwrap();
        assert_eq!(first, replay(&data, ReplayConfig::default()).unwrap());
        assert_eq!(first.opened, 1);
        assert_eq!(first.closed, 1);
        assert_eq!(first.still_open, 0);
        assert!((first.realized_pnl_usdt - 1.64).abs() < 1e-9);
    }

    #[test]
    fn refusal_prevents_entry_without_fake_returns() {
        let data = [tick(0, 101.0, 100.0, false)];
        let result = replay(&data, ReplayConfig::default()).unwrap();
        assert_eq!(result.opened, 0);
        assert_eq!(result.cash_usdt, 1_000.0);
        assert_eq!(result.realized_pnl_usdt, 0.0);
    }

    #[test]
    fn strict_clock_order_rejects_future_and_duplicate_ticks() {
        let data = [tick(5, 101.0, 100.0, true), tick(2, 99.0, 100.0, true)];
        assert!(matches!(replay(&data, ReplayConfig::default()), Err(ReplayError::OutOfOrder)));
        let duplicates = [tick(1, 101.0, 100.0, true), tick(1, 99.0, 100.0, true)];
        assert!(matches!(replay(&duplicates, ReplayConfig::default()), Err(ReplayError::OutOfOrder)));
    }

    #[test]
    fn rejects_invalid_market_data_and_configuration() {
        assert!(matches!(replay(&[], ReplayConfig::default()), Err(ReplayError::EmptyInput)));
        assert!(matches!(
            replay(&[tick(0, f64::NAN, 100.0, true)], ReplayConfig::default()),
            Err(ReplayError::InvalidRow)
        ));
        let mut config = ReplayConfig::default();
        config.estimated_fees_percent = f64::INFINITY;
        assert!(matches!(
            replay(&[tick(0, 101.0, 100.0, true)], config),
            Err(ReplayError::InvalidConfiguration)
        ));
    }

    #[test]
    fn no_forced_liquidation_or_fabricated_unrealized_profit() {
        let data = [tick(0, 101.0, 100.0, true)];
        let result = replay(&data, ReplayConfig::default()).unwrap();
        assert_eq!(result.opened, 1);
        assert_eq!(result.still_open, 1);
        assert_eq!(result.realized_pnl_usdt, 0.0);
        assert_eq!(result.book_equity_usdt, 1000.0);
    }
}
