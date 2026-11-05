//! Explicitly synthetic, in-memory paper-trading sandbox.
//! No credentials, market API, network client, broker transport or real-order path.
use std::collections::VecDeque;
use std::sync::Mutex;

const START_CENTS: u64 = 1_000_000; // 10 000.00 fictitious USDT
const FEE_BPS: u64 = 36; // 0.36% per simulated transaction
const MAX_NOTIONAL_CENTS: u64 = 250_000; // 2 500.00 USDT risk cap per order
const MAX_QTY_MILLI: u32 = 100_000;
const MAX_EVENTS: usize = 20;
const COINS: [(&str, u64); 3] = [
    ("BTC", 6_500_000), // 65 000.00 USDT (synthetic, deliberately fixed)
    ("ETH", 300_000),   //  3 000.00 USDT (synthetic, deliberately fixed)
    ("SOL", 15_000),    //    150.00 USDT (synthetic, deliberately fixed)
];

struct PaperState {
    cash_cents: u64,
    positions_milli: [u32; 3],
    seq: u64,
    events: VecDeque<String>,
}

impl Default for PaperState {
    fn default() -> Self {
        Self {
            cash_cents: START_CENTS,
            positions_milli: [0; 3],
            seq: 0,
            events: VecDeque::new(),
        }
    }
}

/// A disposable portfolio owned by this Rust process, never by WebView storage.
#[derive(Default)]
pub struct PaperSandbox {
    state: Mutex<PaperState>,
}

impl PaperSandbox {
    pub fn reset(&self) -> Vec<String> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        *state = PaperState::default();
        snapshot(&state)
    }

    pub fn snapshot(&self) -> Vec<String> {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        snapshot(&state)
    }

    pub fn trade(
        &self,
        symbol: &str,
        side: &str,
        quantity_milli: u32,
    ) -> Result<Vec<String>, &'static str> {
        let index = COINS
            .iter()
            .position(|(name, _)| *name == symbol)
            .ok_or("INVALID_PAPER_INSTRUMENT")?;
        if side != "BUY" && side != "SELL" {
            return Err("INVALID_PAPER_SIDE");
        }
        if quantity_milli == 0 || quantity_milli > MAX_QTY_MILLI {
            return Err("INVALID_PAPER_QUANTITY");
        }
        let gross = COINS[index]
            .1
            .checked_mul(u64::from(quantity_milli))
            .ok_or("PAPER_ARITHMETIC_ERROR")?
            / 1_000;
        if gross == 0 || gross > MAX_NOTIONAL_CENTS {
            return Err("PAPER_RISK_LIMIT");
        }
        let fee = gross
            .checked_mul(FEE_BPS)
            .ok_or("PAPER_ARITHMETIC_ERROR")?
            .checked_add(9_999)
            .ok_or("PAPER_ARITHMETIC_ERROR")?
            / 10_000;
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if side == "BUY" {
            let total = gross.checked_add(fee).ok_or("PAPER_ARITHMETIC_ERROR")?;
            if total > state.cash_cents {
                return Err("PAPER_INSUFFICIENT_FUNDS");
            }
            let next = state.positions_milli[index]
                .checked_add(quantity_milli)
                .ok_or("PAPER_ARITHMETIC_ERROR")?;
            state.cash_cents -= total;
            state.positions_milli[index] = next;
        } else {
            if quantity_milli > state.positions_milli[index] {
                return Err("PAPER_INSUFFICIENT_POSITION");
            }
            let proceeds = gross.checked_sub(fee).ok_or("PAPER_ARITHMETIC_ERROR")?;
            let next = state.cash_cents.checked_add(proceeds).ok_or("PAPER_ARITHMETIC_ERROR")?;
            state.cash_cents = next;
            state.positions_milli[index] -= quantity_milli;
        }
        state.seq = state.seq.checked_add(1).ok_or("PAPER_ARITHMETIC_ERROR")?;
        if state.events.len() == MAX_EVENTS {
            state.events.pop_front();
        }
        let seq = state.seq;
        state.events.push_back(format!(
            "event|{seq}|{side}|{symbol}|{quantity_milli}|{gross}|{fee}"
        ));
        Ok(snapshot(&state))
    }
}

fn snapshot(state: &PaperState) -> Vec<String> {
    let mut rows = vec![
        "mode|PAPER_ONLY".to_owned(),
        format!("cash_cents|{}", state.cash_cents),
        format!("fee_bps|{FEE_BPS}"),
        "price_type|SYNTHETIC_FIXED".to_owned(),
    ];
    let mut equity = state.cash_cents;
    for (index, (symbol, price_cents)) in COINS.iter().enumerate() {
        let value = price_cents * u64::from(state.positions_milli[index]) / 1_000;
        equity += value;
        rows.push(format!("price_cents|{symbol}|{price_cents}"));
        rows.push(format!("position_milli|{symbol}|{}", state.positions_milli[index]));
    }
    rows.insert(2, format!("equity_cents|{equity}"));
    rows.extend(state.events.iter().rev().cloned());
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_orders_charge_fees_and_reset_without_network() {
        let paper = PaperSandbox::default();
        let buy = paper.trade("BTC", "BUY", 10).unwrap();
        assert!(buy.iter().any(|x| x == "cash_cents|934766")); // 650.00 + 2.34
        assert!(buy.iter().any(|x| x == "position_milli|BTC|10"));
        let sell = paper.trade("BTC", "SELL", 10).unwrap();
        assert!(sell.iter().any(|x| x == "cash_cents|999532")); // 4.68 in fees
        assert!(sell.iter().any(|x| x == "position_milli|BTC|0"));
        assert!(paper.reset().iter().any(|x| x == "cash_cents|1000000"));
    }

    #[test]
    fn rejects_invalid_and_oversized_orders_without_mutating_portfolio() {
        let paper = PaperSandbox::default();
        let start = paper.snapshot();
        for (symbol, side, qty) in [
            ("DOGE", "BUY", 1),
            ("BTC", "HOLD", 1),
            ("BTC", "BUY", 0),
            ("BTC", "BUY", u32::MAX),
            ("BTC", "BUY", 1_000),
            ("ETH", "SELL", 1),
        ] {
            assert!(paper.trade(symbol, side, qty).is_err());
            assert_eq!(paper.snapshot(), start);
        }
    }

    #[test]
    fn events_are_bounded_and_always_paper_only() {
        let paper = PaperSandbox::default();
        for _ in 0..35 {
            paper.trade("SOL", "BUY", 1).unwrap();
            paper.trade("SOL", "SELL", 1).unwrap();
        }
        let rows = paper.snapshot();
        assert_eq!(rows.iter().filter(|r| r.starts_with("event|")).count(), MAX_EVENTS);
        assert_eq!(rows.first().map(String::as_str), Some("mode|PAPER_ONLY"));
    }
}
