use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};

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
        assert!(confirmation_seconds >= 0.0, "confirmation_seconds must be non-negative");
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

        let elapsed = now.signed_duration_since(*candidate_since).num_milliseconds() as f64 / 1000.0;
        if elapsed + 1e-9 < self.confirmation_seconds {
            return false;
        }

        self.candidate_since.remove(symbol);
        self.active.insert(symbol.to_string());
        self.next_activation_id += 1;
        self.activation_ids.insert(symbol.to_string(), self.next_activation_id);
        true
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
}
