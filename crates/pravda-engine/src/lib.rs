pub mod backtest;
pub mod reg_guard;
pub mod research_risk;
pub use backtest::{replay, ReplayConfig, ReplayError, ReplayResult, ReplayTick};

use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct AlertGate {
    confirmation_seconds: f64,
    candidate_since: HashMap<String, Instant>,
    active: HashSet<String>,
    activation_ids: HashMap<String, u64>,
    next_activation_id: u64,
}

impl AlertGate {
    pub fn new(confirmation_seconds: f64) -> Self {
        assert!(
            confirmation_seconds.is_finite() && confirmation_seconds >= 0.0,
            "confirmation_seconds must be non-negative"
        );
        Self {
            confirmation_seconds,
            candidate_since: HashMap::new(),
            active: HashSet::new(),
            activation_ids: HashMap::new(),
            next_activation_id: 0,
        }
    }

    pub fn reset_candidate(&mut self, symbol: &str) {
        self.candidate_since.remove(symbol);
    }

    pub fn activation_id(&self, symbol: &str) -> Option<u64> {
        if self.active.contains(symbol) {
            self.activation_ids.get(symbol).copied()
        } else {
            None
        }
    }

    pub fn cancel_activation(&mut self, symbol: &str, activation_id: u64) -> bool {
        if self.activation_id(symbol) == Some(activation_id) {
            self.active.remove(symbol);
            self.activation_ids.remove(symbol);
            true
        } else {
            false
        }
    }

    pub fn is_active(&self, symbol: &str) -> bool {
        self.active.contains(symbol)
    }

    pub fn update(
        &mut self,
        symbol: &str,
        profitable: bool,
        spread: f64,
        rearm_threshold: f64,
        now: Instant,
    ) -> bool {
        if !spread.is_finite()
            || spread < 0.0
            || !rearm_threshold.is_finite()
            || rearm_threshold < 0.0
        {
            self.reset_candidate(symbol);
            return false;
        }
        if self.active.contains(symbol) {
            if spread <= rearm_threshold {
                self.active.remove(symbol);
                self.activation_ids.remove(symbol);
            } else {
                return false;
            }
        }

        if !profitable {
            self.reset_candidate(symbol);
            return false;
        }

        let candidate_since = self
            .candidate_since
            .entry(symbol.to_string())
            .or_insert(now);

        let Some(elapsed) = now.checked_duration_since(*candidate_since) else {
            *candidate_since = now;
            return false;
        };
        if elapsed.as_secs_f64() + 1e-9 < self.confirmation_seconds {
            return false;
        }

        self.candidate_since.remove(symbol);
        self.active.insert(symbol.to_string());
        self.next_activation_id += 1;
        self.activation_ids
            .insert(symbol.to_string(), self.next_activation_id);
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperMode {
    Fixed,
    Portfolio,
}

#[derive(Debug, Clone, Copy)]
enum PaperClock {
    Recorded,
    Live(Instant),
}

#[derive(Debug, Clone, Copy)]
enum PaperTime {
    Recorded(DateTime<Utc>),
    Live(DateTime<Utc>, Instant),
}

#[derive(Debug, Clone)]
pub struct PaperPosition {
    pub symbol: String,
    pub notional_usdt: f64,
    pub entry_signed_spread_percent: f64,
    pub opened_at: DateTime<Utc>,
    clock: PaperClock,
}

pub struct PaperSimulator {
    pub mode: PaperMode,
    pub starting_balance_usdt: f64,
    pub cash_usdt: f64,
    pub position_percent: f64,
    pub fixed_position_usdt: f64,
    pub exit_spread_percent: f64,
    pub max_hold_seconds: f64,
    pub estimated_fees_percent: f64,
    pub positions: HashMap<String, PaperPosition>,
    pub realized_pnl_usdt: f64,
    pub completed_positions: u64,
}

impl PaperSimulator {
    pub fn new(
        mode: PaperMode,
        starting_balance_usdt: f64,
        position_percent: f64,
        fixed_position_usdt: f64,
        exit_spread_percent: f64,
        max_hold_seconds: f64,
        estimated_fees_percent: f64,
    ) -> Self {
        Self {
            mode,
            starting_balance_usdt,
            cash_usdt: starting_balance_usdt,
            position_percent,
            fixed_position_usdt,
            exit_spread_percent,
            max_hold_seconds,
            estimated_fees_percent,
            positions: HashMap::new(),
            realized_pnl_usdt: 0.0,
            completed_positions: 0,
        }
    }

    fn position_size(&self) -> f64 {
        match self.mode {
            PaperMode::Fixed => self.fixed_position_usdt.min(self.cash_usdt),
            PaperMode::Portfolio => self.cash_usdt * self.position_percent / 100.0,
        }
    }

    pub fn signed_spread_percent(kraken_price: f64, binance_price: f64) -> f64 {
        (kraken_price - binance_price) / binance_price * 100.0
    }

    /// Recorded-time entry point retained for historical replay and parity.
    /// Live callers should use `update_live` for monotonic holding durations.
    pub fn update(
        &mut self,
        symbol: &str,
        kraken_price: f64,
        binance_price: f64,
        opportunity_confirmed: bool,
        now: DateTime<Utc>,
    ) -> Option<String> {
        self.update_with_execution_approval(
            symbol,
            kraken_price,
            binance_price,
            opportunity_confirmed,
            true,
            now,
        )
    }

    /// Legacy Python parity: an explicit rejection must prevent new entries,
    /// while never blocking an existing position from being closed.
    pub fn update_with_execution_approval(
        &mut self,
        symbol: &str,
        kraken_price: f64,
        binance_price: f64,
        opportunity_confirmed: bool,
        execution_approved: bool,
        now: DateTime<Utc>,
    ) -> Option<String> {
        self.update_at(
            symbol,
            kraken_price,
            binance_price,
            opportunity_confirmed,
            execution_approved,
            PaperTime::Recorded(now),
        )
    }

    /// Live simulation uses wall time for display and monotonic time for age.
    /// A position opened in one clock mode cannot be updated in the other.
    pub fn update_live(
        &mut self,
        symbol: &str,
        kraken_price: f64,
        binance_price: f64,
        opportunity_confirmed: bool,
        now: DateTime<Utc>,
        monotonic_now: Instant,
    ) -> Option<String> {
        self.update_at(
            symbol,
            kraken_price,
            binance_price,
            opportunity_confirmed,
            true,
            PaperTime::Live(now, monotonic_now),
        )
    }

    fn update_at(
        &mut self,
        symbol: &str,
        kraken_price: f64,
        binance_price: f64,
        opportunity_confirmed: bool,
        execution_approved: bool,
        now: PaperTime,
    ) -> Option<String> {
        if !kraken_price.is_finite()
            || !binance_price.is_finite()
            || kraken_price <= 0.0
            || binance_price <= 0.0
        {
            return None;
        }
        let signed_spread = Self::signed_spread_percent(kraken_price, binance_price);
        if !signed_spread.is_finite() {
            return None;
        }

        if let Some(pos) = self.positions.get(symbol) {
            let entry_direction = if pos.entry_signed_spread_percent > 0.0 {
                1.0
            } else {
                -1.0
            };
            let remaining_signed_spread = signed_spread * entry_direction;
            let age_seconds = match (pos.clock, now) {
                (PaperClock::Recorded, PaperTime::Recorded(timestamp)) => {
                    timestamp.signed_duration_since(pos.opened_at).num_seconds() as f64
                }
                (PaperClock::Live(started), PaperTime::Live(_, current)) => {
                    current.checked_duration_since(started)?.as_secs_f64()
                }
                _ => return None,
            };
            let should_close = remaining_signed_spread <= self.exit_spread_percent
                || age_seconds >= self.max_hold_seconds;
            if should_close {
                let gross_pnl = (pos.entry_signed_spread_percent.abs() - remaining_signed_spread)
                    * pos.notional_usdt
                    / 100.0;
                let fees = pos.notional_usdt * self.estimated_fees_percent / 100.0;
                let net_pnl = gross_pnl - fees;
                self.cash_usdt += pos.notional_usdt + net_pnl;
                self.realized_pnl_usdt += net_pnl;
                self.completed_positions += 1;
                self.positions.remove(symbol);
                return Some("closed".to_string());
            }
            return None;
        }

        if !opportunity_confirmed || !execution_approved {
            return None;
        }

        let notional = self.position_size();
        if notional <= 0.0 {
            return None;
        }
        self.cash_usdt -= notional;
        self.positions.insert(
            symbol.to_string(),
            PaperPosition {
                symbol: symbol.to_string(),
                notional_usdt: notional,
                entry_signed_spread_percent: signed_spread,
                opened_at: match now {
                    PaperTime::Recorded(timestamp) | PaperTime::Live(timestamp, _) => timestamp,
                },
                clock: match now {
                    PaperTime::Recorded(_) => PaperClock::Recorded,
                    PaperTime::Live(_, instant) => PaperClock::Live(instant),
                },
            },
        );
        Some("opened".to_string())
    }

    pub fn equity_usdt(&self) -> f64 {
        self.cash_usdt
            + self
                .positions
                .values()
                .map(|p| p.notional_usdt)
                .sum::<f64>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn reversal_preserves_signed_paper_return() {
        let mut sim =
            PaperSimulator::new(PaperMode::Fixed, 1000.0, 10.0, 100.0, 0.05, 3600.0, 0.36);
        let now = Utc::now();
        assert!(sim.update("BTC/USDT", 101.0, 100.0, true, now).is_some());
        assert!(sim.update("BTC/USDT", 99.0, 100.0, false, now).is_some());
        assert!((sim.realized_pnl_usdt - 1.64).abs() < 1e-10);
    }

    #[test]
    fn invalid_prices_do_not_mutate_paper_state() {
        let mut sim =
            PaperSimulator::new(PaperMode::Fixed, 1000.0, 10.0, 100.0, 0.05, 3600.0, 0.36);
        for price in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
            assert!(sim
                .update("BTC/USDT", price, 100.0, true, Utc::now())
                .is_none());
        }
        assert_eq!(sim.cash_usdt, 1000.0);
        assert!(sim.positions.is_empty());
    }

    #[test]
    fn test_alert_gate_basic_flow() {
        let mut gate = AlertGate::new(3.0);
        let now = Instant::now();
        assert!(!gate.update("BTC/USDT", true, 0.01, 0.005, now));
        let later = now + std::time::Duration::from_secs(4);
        assert!(gate.update("BTC/USDT", true, 0.01, 0.005, later));
        assert!(gate.is_active("BTC/USDT"));
    }

    #[test]
    fn confirmation_uses_monotonic_elapsed_time_and_resets_on_backward_input() {
        let mut gate = AlertGate::new(3.0);
        let start = Instant::now();
        let seconds = std::time::Duration::from_secs;
        assert!(!gate.update("BTC/USDT", true, 0.5, 0.005, start));
        assert!(!gate.update("BTC/USDT", true, 0.5, 0.005, start + seconds(2)));
        assert!(gate.update("BTC/USDT", true, 0.5, 0.005, start + seconds(3)));
        let mut backwards = AlertGate::new(3.0);
        assert!(!backwards.update("BTC/USDT", true, 0.5, 0.005, start + seconds(10)));
        assert!(!backwards.update("BTC/USDT", true, 0.5, 0.005, start + seconds(9)));
        assert!(!backwards.update("BTC/USDT", true, 0.5, 0.005, start + seconds(11)));
        assert!(backwards.update("BTC/USDT", true, 0.5, 0.005, start + seconds(12)));
    }

    #[test]
    fn test_paper_simulator_open_close() {
        let mut sim = PaperSimulator::new(PaperMode::Fixed, 1000.0, 10.0, 100.0, 0.05, 3600.0, 0.1);
        let now = Utc::now();
        assert_eq!(
            sim.update("BTC/USDT", 101.0, 100.0, true, now),
            Some("opened".to_string())
        );
        let later = now + Duration::seconds(10);
        // spread closed
        assert_eq!(
            sim.update("BTC/USDT", 100.01, 100.0, false, later),
            Some("closed".to_string())
        );
    }

    #[test]
    fn live_paper_max_hold_must_not_follow_wall_clock_jumps() {
        let mut sim =
            PaperSimulator::new(PaperMode::Fixed, 1000.0, 10.0, 100.0, 0.05, 3600.0, 0.36);
        let wall = Utc::now();
        let start = Instant::now();
        let seconds = std::time::Duration::from_secs;
        assert_eq!(
            sim.update_live("BTC/USDT", 101.0, 100.0, true, wall, start),
            Some("opened".to_owned())
        );
        assert_eq!(
            sim.update_live(
                "BTC/USDT",
                101.0,
                100.0,
                false,
                wall + Duration::hours(2),
                start + seconds(1)
            ),
            None,
            "clock adjustment must not close a live position early"
        );
        assert_eq!(
            sim.update_live(
                "BTC/USDT",
                101.0,
                100.0,
                false,
                wall - Duration::hours(2),
                start + seconds(3599)
            ),
            None
        );
        assert_eq!(
            sim.update_live(
                "BTC/USDT",
                101.0,
                100.0,
                false,
                wall - Duration::hours(2),
                start + seconds(3600)
            ),
            Some("closed".to_owned())
        );
    }

    #[test]
    fn live_and_recorded_paper_clocks_cannot_be_mixed() {
        let mut live =
            PaperSimulator::new(PaperMode::Fixed, 1000.0, 10.0, 100.0, 0.05, 3600.0, 0.36);
        let wall = Utc::now();
        let start = Instant::now();
        assert_eq!(
            live.update_live("BTC/USDT", 101.0, 100.0, true, wall, start),
            Some("opened".to_owned())
        );
        assert_eq!(
            live.update("BTC/USDT", 100.0, 100.0, false, wall + Duration::hours(2)),
            None
        );
        assert!(live.positions.contains_key("BTC/USDT"));

        let mut recorded =
            PaperSimulator::new(PaperMode::Fixed, 1000.0, 10.0, 100.0, 0.05, 3600.0, 0.36);
        assert_eq!(
            recorded.update("BTC/USDT", 101.0, 100.0, true, wall),
            Some("opened".to_owned())
        );
        assert_eq!(
            recorded.update_live("BTC/USDT", 100.0, 100.0, false, wall, start),
            None
        );
        assert!(recorded.positions.contains_key("BTC/USDT"));
    }

    #[test]
    fn backwards_live_timestamp_does_not_close_a_position() {
        let mut sim =
            PaperSimulator::new(PaperMode::Fixed, 1000.0, 10.0, 100.0, 0.05, 3600.0, 0.36);
        let wall = Utc::now();
        let start = Instant::now();
        let seconds = std::time::Duration::from_secs;
        assert_eq!(
            sim.update_live("BTC/USDT", 101.0, 100.0, true, wall, start + seconds(10)),
            Some("opened".to_owned())
        );
        assert_eq!(
            sim.update_live("BTC/USDT", 100.0, 100.0, false, wall, start + seconds(9)),
            None
        );
        assert!(sim.positions.contains_key("BTC/USDT"));
    }
}
