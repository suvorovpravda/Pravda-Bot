use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct AlertGate {
    confirmation_seconds: f64,
    candidate_since: HashMap<String, DateTime<Utc>>,
    active: HashSet<String>,
    activation_ids: HashMap<String, u64>,
    next_activation_id: u64,
}

impl AlertGate {
    pub fn new(confirmation_seconds: f64) -> Self {
        assert!(
            confirmation_seconds >= 0.0,
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
        now: DateTime<Utc>,
    ) -> bool {
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
            .or_insert_with(|| now);

        let elapsed = now
            .signed_duration_since(*candidate_since)
            .num_milliseconds() as f64
            / 1000.0;
        if elapsed + 1e-9 < self.confirmation_seconds {
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

#[derive(Debug, Clone)]
pub struct PaperPosition {
    pub symbol: String,
    pub notional_usdt: f64,
    pub entry_signed_spread_percent: f64,
    pub opened_at: DateTime<Utc>,
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

    pub fn update(
        &mut self,
        symbol: &str,
        kraken_price: f64,
        binance_price: f64,
        opportunity_confirmed: bool,
        now: DateTime<Utc>,
    ) -> Option<String> {
        let signed_spread = Self::signed_spread_percent(kraken_price, binance_price);

        if let Some(pos) = self.positions.get(symbol) {
            let entry_direction = if pos.entry_signed_spread_percent > 0.0 {
                1.0
            } else {
                -1.0
            };
            let remaining_signed_spread = signed_spread * entry_direction;
            let age_seconds = now.signed_duration_since(pos.opened_at).num_seconds() as f64;
            let should_close = remaining_signed_spread <= self.exit_spread_percent
                || age_seconds >= self.max_hold_seconds;
            if should_close {
                let gross_pnl = (pos.entry_signed_spread_percent.abs()
                    - remaining_signed_spread.abs())
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

        if !opportunity_confirmed {
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
                opened_at: now,
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
    fn test_alert_gate_basic_flow() {
        let mut gate = AlertGate::new(3.0);
        let now = Utc::now();
        assert!(!gate.update("BTC/USDT", true, 0.01, 0.005, now));
        let later = now + Duration::seconds(4);
        assert!(gate.update("BTC/USDT", true, 0.01, 0.005, later));
        assert!(gate.is_active("BTC/USDT"));
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
}
