//! Offline Revolut X order state ledger.
//!
//! This is an in-memory model for deterministic reconciliation tests. It is
//! deliberately NOT a durable idempotency store and provides no network
//! transport. A production order sender must atomically persist reservations,
//! recover unknown orders and obtain explicit operator authorization first.
use std::collections::HashMap;
use thiserror::Error;

use crate::revolut_x_orders::PreparedLimitOrder;

const MAX_TRACKED_ORDERS: usize = 10_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderState {
    Prepared,
    Submitted,
    Accepted,
    PartiallyFilled,
    CancelRequested,
    Filled,
    Cancelled,
    Rejected,
    /// The exchange outcome is unknown. Never submit the order again merely
    /// because a network connection has failed.
    Unknown,
}

impl OrderState {
    fn terminal(self) -> bool {
        matches!(self, Self::Filled | Self::Cancelled | Self::Rejected)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExchangeOrderStatus {
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderRecord {
    pub client_order_id: String,
    pub exchange_order_id: Option<String>,
    pub state: OrderState,
    /// Fixed-point base quantity in units of 10^-8.
    pub base_units: u128,
    /// Cumulative exchange-confirmed quantity in units of 10^-8.
    pub filled_base_units: u128,
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum LedgerError {
    #[error("order submission disabled by emergency stop")]
    Killed,
    #[error("duplicate client order ID; reuse is never permitted")]
    Duplicate,
    #[error("order tracking capacity exhausted")]
    Capacity,
    #[error("unknown client order ID")]
    NotFound,
    #[error("order state transition is not allowed")]
    InvalidTransition,
    #[error("cumulative fill is invalid or decreases")]
    InvalidFill,
    #[error("exchange order identity changed")]
    ExchangeIdMismatch,
    #[error("exchange reconciliation snapshot is inconsistent")]
    Reconciliation,
}

/// This ledger has no exchange keys, HTTP client or persistent storage.
/// It demonstrates fail-closed transitions; do not use it as a live sender.
#[derive(Default)]
pub struct OrderLedger {
    killed: bool,
    orders: HashMap<String, OrderRecord>,
}

impl OrderLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Emergency stop blocks all new reservations and submissions. Existing
    /// exchange events may still be recorded, including fills after shutdown.
    pub fn emergency_stop(&mut self) {
        self.killed = true;
    }

    pub fn is_stopped(&self) -> bool {
        self.killed
    }

    pub fn reserve(&mut self, prepared: &PreparedLimitOrder) -> Result<(), LedgerError> {
        if self.killed {
            return Err(LedgerError::Killed);
        }
        let id = prepared.client_order_id();
        if self.orders.contains_key(id) {
            return Err(LedgerError::Duplicate);
        }
        if self.orders.len() >= MAX_TRACKED_ORDERS {
            return Err(LedgerError::Capacity);
        }
        self.orders.insert(
            id.to_owned(),
            OrderRecord {
                client_order_id: id.to_owned(),
                exchange_order_id: None,
                state: OrderState::Prepared,
                base_units: prepared.base_units(),
                filled_base_units: 0,
            },
        );
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&OrderRecord> {
        self.orders.get(id)
    }

    pub fn mark_submitted(&mut self, id: &str) -> Result<(), LedgerError> {
        if self.killed {
            return Err(LedgerError::Killed);
        }
        let order = self.orders.get_mut(id).ok_or(LedgerError::NotFound)?;
        if order.state != OrderState::Prepared {
            return Err(LedgerError::InvalidTransition);
        }
        order.state = OrderState::Submitted;
        Ok(())
    }

    pub fn accept(&mut self, id: &str, exchange_id: &str) -> Result<(), LedgerError> {
        if exchange_id.is_empty() || exchange_id.len() > 128 {
            return Err(LedgerError::ExchangeIdMismatch);
        }
        let order = self.orders.get_mut(id).ok_or(LedgerError::NotFound)?;
        if order.state != OrderState::Submitted {
            return Err(LedgerError::InvalidTransition);
        }
        order.exchange_order_id = Some(exchange_id.to_owned());
        order.state = OrderState::Accepted;
        Ok(())
    }

    pub fn cumulative_fill(&mut self, id: &str, filled: u128) -> Result<(), LedgerError> {
        let order = self.orders.get_mut(id).ok_or(LedgerError::NotFound)?;
        if !matches!(
            order.state,
            OrderState::Accepted | OrderState::PartiallyFilled | OrderState::CancelRequested
        ) {
            return Err(LedgerError::InvalidTransition);
        }
        if filled < order.filled_base_units || filled > order.base_units {
            return Err(LedgerError::InvalidFill);
        }
        if filled == order.filled_base_units {
            return Ok(());
        }
        order.filled_base_units = filled;
        if filled == order.base_units {
            order.state = OrderState::Filled;
        } else if order.state != OrderState::CancelRequested {
            order.state = OrderState::PartiallyFilled;
        }
        Ok(())
    }

    pub fn request_cancel(&mut self, id: &str) -> Result<(), LedgerError> {
        let order = self.orders.get_mut(id).ok_or(LedgerError::NotFound)?;
        if !matches!(
            order.state,
            OrderState::Accepted | OrderState::PartiallyFilled
        ) {
            return Err(LedgerError::InvalidTransition);
        }
        order.state = OrderState::CancelRequested;
        Ok(())
    }

    pub fn mark_disconnected(&mut self, id: &str) -> Result<(), LedgerError> {
        let order = self.orders.get_mut(id).ok_or(LedgerError::NotFound)?;
        if order.state == OrderState::Prepared || order.state.terminal() {
            return Err(LedgerError::InvalidTransition);
        }
        order.state = OrderState::Unknown;
        Ok(())
    }

    /// Only a trusted exchange status snapshot may resolve an unknown
    /// result. Untrusted/missing snapshots are not treated as rejected orders.
    pub fn reconcile_unknown(
        &mut self,
        id: &str,
        exchange_id: &str,
        status: ExchangeOrderStatus,
        cumulative: u128,
    ) -> Result<(), LedgerError> {
        let order = self.orders.get_mut(id).ok_or(LedgerError::NotFound)?;
        if order.state != OrderState::Unknown {
            return Err(LedgerError::InvalidTransition);
        }
        if exchange_id.is_empty() || exchange_id.len() > 128 {
            return Err(LedgerError::ExchangeIdMismatch);
        }
        if order
            .exchange_order_id
            .as_deref()
            .is_some_and(|known| known != exchange_id)
        {
            return Err(LedgerError::ExchangeIdMismatch);
        }
        if cumulative < order.filled_base_units || cumulative > order.base_units {
            return Err(LedgerError::InvalidFill);
        }
        let next = match status {
            ExchangeOrderStatus::Open if cumulative == 0 => OrderState::Accepted,
            ExchangeOrderStatus::Open | ExchangeOrderStatus::PartiallyFilled
                if cumulative > 0 && cumulative < order.base_units =>
            {
                OrderState::PartiallyFilled
            }
            ExchangeOrderStatus::Filled if cumulative == order.base_units => OrderState::Filled,
            ExchangeOrderStatus::Cancelled => OrderState::Cancelled,
            ExchangeOrderStatus::Rejected if cumulative == 0 => OrderState::Rejected,
            _ => return Err(LedgerError::Reconciliation),
        };
        order.exchange_order_id = Some(exchange_id.to_owned());
        order.filled_base_units = cumulative;
        order.state = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::revolut_x_orders::{prepare_limit_order, LimitOrderIntent, OrderRiskLimits, Side};

    const ID: &str = "3b364427-1f4f-4f66-9935-86b6fb115d26";

    fn prepared() -> PreparedLimitOrder {
        let order = LimitOrderIntent {
            client_order_id: ID.into(),
            symbol: "BTC-USD".into(),
            side: Side::Buy,
            base_size: "0.01".into(),
            price: "90000".into(),
        };
        prepare_limit_order(
            &order,
            OrderRiskLimits {
                max_order_quote_units: 1_000 * 100_000_000,
                remaining_daily_quote_units: 1_000 * 100_000_000,
                last_market_timestamp_ms: 999_000,
                max_market_age_ms: 2_000,
                reference_price: "90000",
                max_price_deviation_bps: 100,
            },
            1_000_000,
            true,
        )
        .unwrap()
    }

    #[test]
    fn duplicate_reservation_is_rejected_before_submission() {
        let mut ledger = OrderLedger::new();
        ledger.reserve(&prepared()).unwrap();
        assert_eq!(ledger.reserve(&prepared()), Err(LedgerError::Duplicate));
        assert_eq!(ledger.get(ID).unwrap().state, OrderState::Prepared);
    }

    #[test]
    fn cumulative_partial_fills_are_idempotent_and_never_overfill() {
        let mut ledger = OrderLedger::new();
        ledger.reserve(&prepared()).unwrap();
        ledger.mark_submitted(ID).unwrap();
        ledger.accept(ID, "exchange-001").unwrap();
        ledger.cumulative_fill(ID, 250_000).unwrap();
        ledger.cumulative_fill(ID, 250_000).unwrap();
        assert_eq!(ledger.get(ID).unwrap().filled_base_units, 250_000);
        assert_eq!(
            ledger.cumulative_fill(ID, 249_999),
            Err(LedgerError::InvalidFill)
        );
        assert_eq!(
            ledger.cumulative_fill(ID, 1_000_001),
            Err(LedgerError::InvalidFill)
        );
        ledger.cumulative_fill(ID, 1_000_000).unwrap();
        assert_eq!(ledger.get(ID).unwrap().state, OrderState::Filled);
        assert_eq!(
            ledger.request_cancel(ID),
            Err(LedgerError::InvalidTransition)
        );
    }

    #[test]
    fn disconnect_requires_exchange_reconciliation_never_resubmission() {
        let mut ledger = OrderLedger::new();
        ledger.reserve(&prepared()).unwrap();
        ledger.mark_submitted(ID).unwrap();
        ledger.mark_disconnected(ID).unwrap();
        assert_eq!(ledger.get(ID).unwrap().state, OrderState::Unknown);
        assert_eq!(
            ledger.mark_submitted(ID),
            Err(LedgerError::InvalidTransition)
        );
        assert_eq!(
            ledger.reconcile_unknown(ID, "exchange-001", ExchangeOrderStatus::Filled, 1),
            Err(LedgerError::Reconciliation)
        );
        assert_eq!(ledger.get(ID).unwrap().state, OrderState::Unknown);
        ledger
            .reconcile_unknown(
                ID,
                "exchange-001",
                ExchangeOrderStatus::PartiallyFilled,
                200_000,
            )
            .unwrap();
        assert_eq!(ledger.get(ID).unwrap().state, OrderState::PartiallyFilled);
        assert_eq!(ledger.get(ID).unwrap().filled_base_units, 200_000);
    }

    #[test]
    fn cancellation_and_kill_switch_do_not_hide_late_fills() {
        let mut ledger = OrderLedger::new();
        ledger.reserve(&prepared()).unwrap();
        ledger.mark_submitted(ID).unwrap();
        ledger.accept(ID, "exchange-001").unwrap();
        ledger.request_cancel(ID).unwrap();
        ledger.emergency_stop();
        assert_eq!(ledger.reserve(&prepared()), Err(LedgerError::Killed));
        ledger.cumulative_fill(ID, 1_000_000).unwrap();
        assert_eq!(ledger.get(ID).unwrap().state, OrderState::Filled);
    }

    #[test]
    fn exchange_id_mismatch_and_fill_rollback_fail_closed() {
        let mut ledger = OrderLedger::new();
        ledger.reserve(&prepared()).unwrap();
        ledger.mark_submitted(ID).unwrap();
        ledger.accept(ID, "original").unwrap();
        ledger.cumulative_fill(ID, 400_000).unwrap();
        ledger.mark_disconnected(ID).unwrap();
        assert_eq!(
            ledger.reconcile_unknown(ID, "other", ExchangeOrderStatus::Cancelled, 400_000),
            Err(LedgerError::ExchangeIdMismatch)
        );
        assert_eq!(
            ledger.reconcile_unknown(ID, "original", ExchangeOrderStatus::Cancelled, 100_000),
            Err(LedgerError::InvalidFill)
        );
        ledger
            .reconcile_unknown(ID, "original", ExchangeOrderStatus::Cancelled, 500_000)
            .unwrap();
        assert_eq!(ledger.get(ID).unwrap().state, OrderState::Cancelled);
        assert_eq!(ledger.get(ID).unwrap().filled_base_units, 500_000);
    }
}
