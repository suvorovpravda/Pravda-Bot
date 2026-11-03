//! Offline reconciliation primitives for Revolut X. No network, credentials or order submission.
//! Unknown, stale or regressing exchange states fail closed and require operator review.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VenueState {
    New,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
    Expired,
}
impl VenueState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Filled | Self::Cancelled | Self::Rejected | Self::Expired)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VenueOrderSnapshot {
    pub client_order_id: String,
    pub venue_order_id: String,
    pub state: VenueState,
    /// Cumulative filled quantity in integer exchange base-step units.
    pub filled_steps: u128,
    pub observed_at_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReconciliationError {
    IdentityMismatch,
    StaleSnapshot,
    FillRegression,
    InvalidTransition,
    UnexpectedFill,
    MissingReconciliation,
}

/// Only accepts monotonic, identity-preserving observations. On uncertain delivery,
/// the caller MUST reconcile by venue ID before attempting any new order.
pub fn reconcile(previous: &VenueOrderSnapshot, next: &VenueOrderSnapshot)
    -> Result<(), ReconciliationError>
{
    if previous.client_order_id != next.client_order_id
        || previous.venue_order_id != next.venue_order_id
        || previous.client_order_id.is_empty()
        || previous.venue_order_id.is_empty()
    {
        return Err(ReconciliationError::IdentityMismatch);
    }
    if next.observed_at_ms <= previous.observed_at_ms {
        return Err(ReconciliationError::StaleSnapshot);
    }
    if next.filled_steps < previous.filled_steps {
        return Err(ReconciliationError::FillRegression);
    }
    if previous.state.is_terminal() && next.state != previous.state {
        return Err(ReconciliationError::InvalidTransition);
    }
    if matches!(next.state, VenueState::New | VenueState::Rejected)
        && next.filled_steps != previous.filled_steps
    {
        return Err(ReconciliationError::UnexpectedFill);
    }
    if previous.state == VenueState::PartiallyFilled && next.state == VenueState::New {
        return Err(ReconciliationError::InvalidTransition);
    }
    Ok(())
}

/// Unknown HTTP outcomes (timeout, disconnect, 429, 5xx) must not trigger
/// an automatic resubmission: first persist and reconcile the original UUID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmissionOutcome {
    Confirmed,
    Rejected,
    Unknown,
}
pub fn may_retry_with_new_id(outcome: SubmissionOutcome) -> bool {
    matches!(outcome, SubmissionOutcome::Rejected)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot(state: VenueState, filled: u128, time: u64) -> VenueOrderSnapshot {
        VenueOrderSnapshot {
            client_order_id: "client-uuid".into(),
            venue_order_id: "venue-uuid".into(),
            state, filled_steps: filled, observed_at_ms: time,
        }
    }
    #[test]
    fn accepts_partial_then_full_fill() {
        assert_eq!(reconcile(&snapshot(VenueState::New, 0, 1), &snapshot(VenueState::PartiallyFilled, 4, 2)), Ok(()));
        assert_eq!(reconcile(&snapshot(VenueState::PartiallyFilled, 4, 2), &snapshot(VenueState::Filled, 10, 3)), Ok(()));
    }
    #[test]
    fn rejects_stale_regression_and_identity_switch() {
        assert_eq!(reconcile(&snapshot(VenueState::New, 0, 2), &snapshot(VenueState::New, 0, 2)), Err(ReconciliationError::StaleSnapshot));
        assert_eq!(reconcile(&snapshot(VenueState::PartiallyFilled, 5, 2), &snapshot(VenueState::Filled, 4, 3)), Err(ReconciliationError::FillRegression));
        let mut changed = snapshot(VenueState::Filled, 5, 3);
        changed.venue_order_id = "different".into();
        assert_eq!(reconcile(&snapshot(VenueState::New, 0, 2), &changed), Err(ReconciliationError::IdentityMismatch));
        assert_eq!(reconcile(&snapshot(VenueState::Filled, 5, 2), &snapshot(VenueState::New, 5, 3)), Err(ReconciliationError::InvalidTransition));
    }
    #[test]
    fn uncertain_transport_never_retries_as_new_order() {
        assert!(!may_retry_with_new_id(SubmissionOutcome::Unknown));
        assert!(!may_retry_with_new_id(SubmissionOutcome::Confirmed));
        assert!(may_retry_with_new_id(SubmissionOutcome::Rejected));
    }
}
