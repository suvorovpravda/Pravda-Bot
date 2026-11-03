//! Durable, strictly offline Revolut X order reservation journal.
//!
//! No API transport, account keys, live mode, HTTP, or payment operations.
//! Intended as a building block for #89, not authorization to send orders.
//! Every mutation is performed in an IMMEDIATE SQLite transaction. After a
//! network ambiguity, an order remains UNKNOWN until authenticated read-only
//! exchange reconciliation; retrying a write request is prohibited.
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, TransactionBehavior};
use std::{path::Path, time::Duration};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum JournalError {
    #[error("persistent journal unavailable")]
    Store(#[from] rusqlite::Error),
    #[error("invalid order identity or size")]
    InvalidInput,
    #[error("client order identity has already been reserved")]
    Duplicate,
    #[error("live order reservation disabled by persistent kill switch")]
    Stopped,
    #[error("order transition rejected or stale")]
    InvalidTransition,
    #[error("order not found")]
    NotFound,
    #[error("inconsistent exchange identity or fill amount")]
    Inconsistent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Reserved,
    Submitting,
    Unknown,
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

impl State {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Reserved => "reserved",
            Self::Submitting => "submitting",
            Self::Unknown => "unknown",
            Self::Open => "open",
            Self::PartiallyFilled => "partial",
            Self::Filled => "filled",
            Self::Cancelled => "cancelled",
            Self::Rejected => "rejected",
        }
    }
    fn parse(value: &str) -> Result<Self, JournalError> {
        match value {
            "reserved" => Ok(Self::Reserved),
            "submitting" => Ok(Self::Submitting),
            "unknown" => Ok(Self::Unknown),
            "open" => Ok(Self::Open),
            "partial" => Ok(Self::PartiallyFilled),
            "filled" => Ok(Self::Filled),
            "cancelled" => Ok(Self::Cancelled),
            "rejected" => Ok(Self::Rejected),
            _ => Err(JournalError::Inconsistent),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reservation {
    pub id: String,
    pub exchange_id: Option<String>,
    pub state: State,
    pub base_units: u128,
    pub filled_units: u128,
}

pub struct OrderJournal {
    connection: Connection,
}

impl OrderJournal {
    /// Only create this storage at an explicit, controlled application path;
    /// never mix private-order accounting with temporary market replay data.
    pub fn open(path: &Path) -> Result<Self, JournalError> {
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS order_journal_control(
               id INTEGER PRIMARY KEY CHECK(id = 1),
               stopped INTEGER NOT NULL CHECK(stopped IN (0,1))
             );
             INSERT OR IGNORE INTO order_journal_control(id,stopped) VALUES(1,1);
             CREATE TABLE IF NOT EXISTS order_journal(
               client_order_id TEXT PRIMARY KEY NOT NULL,
               state TEXT NOT NULL CHECK(state IN
                 ('reserved','submitting','unknown','open','partial','filled','cancelled','rejected')),
               exchange_order_id TEXT,
               base_units TEXT NOT NULL,
               filled_units TEXT NOT NULL
             );",
        )?;
        // Fail-closed across process restarts. A reopened database is stopped
        // until an explicit, separately authorized future activation procedure.
        connection.execute(
            "UPDATE order_journal_control SET stopped=1 WHERE id=1",
            [],
        )?;
        Ok(Self { connection })
    }

    pub fn stopped(&self) -> Result<bool, JournalError> {
        self.connection
            .query_row(
                "SELECT stopped FROM order_journal_control WHERE id=1",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(Into::into)
    }

    pub fn emergency_stop(&mut self) -> Result<(), JournalError> {
        self.connection.execute(
            "UPDATE order_journal_control SET stopped=1 WHERE id=1",
            [],
        )?;
        Ok(())
    }

    /// Deliberately no public enable method: readiness, authorization and
    /// broker risk checks must be independently designed and reviewed.
    pub fn reserve(
        &mut self,
        id: &str,
        base_units: u128,
    ) -> Result<(), JournalError> {
        if id.is_empty()
            || id.len() > 64
            || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || base_units == 0
        {
            return Err(JournalError::InvalidInput);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let stopped: bool = tx.query_row(
            "SELECT stopped FROM order_journal_control WHERE id=1",
            [],
            |row| row.get(0),
        )?;
        if stopped {
            return Err(JournalError::Stopped);
        }
        let result = tx.execute(
            "INSERT INTO order_journal(client_order_id,state,exchange_order_id,base_units,filled_units)
             VALUES(?1,'reserved',NULL,?2,'0')",
            params![id, base_units.to_string()],
        );
        match result {
            Ok(_) => {
                tx.commit()?;
                Ok(())
            }
            Err(rusqlite::Error::SqliteFailure(e, _))
                if e.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Err(JournalError::Duplicate)
            }
            Err(e) => Err(JournalError::Store(e)),
        }
    }

    pub fn get(&self, id: &str) -> Result<Option<Reservation>, JournalError> {
        let raw: Option<(String, Option<String>, String, String)> = self
            .connection
            .query_row(
                "SELECT state,exchange_order_id,base_units,filled_units
                 FROM order_journal WHERE client_order_id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        let Some((state, exchange_id, base, filled)) = raw else {
            return Ok(None);
        };
        Ok(Some(Reservation {
            id: id.to_owned(),
            exchange_id,
            state: State::parse(&state)?,
            base_units: base.parse().map_err(|_| JournalError::Inconsistent)?,
            filled_units: filled.parse().map_err(|_| JournalError::Inconsistent)?,
        }))
    }

    /// Explicitly record uncertainty, not a negative acknowledgement.
    /// Crucially, UNKNOWN is never eligible for a second submission.
    pub fn mark_unknown(&mut self, id: &str) -> Result<(), JournalError> {
        let changed = self.connection.execute(
            "UPDATE order_journal SET state='unknown'
             WHERE client_order_id=?1 AND state IN ('submitting','open','partial')",
            [id],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(JournalError::InvalidTransition)
        }
    }

    /// The local journal records intent before *any* future network send.
    /// A durable submission acknowledgement is not assumed.
    pub fn mark_submitting(&mut self, id: &str) -> Result<(), JournalError> {
        // Deliberately disabled until live order gates are independently
        // reviewed. In this module, no transport operation is possible.
        let _ = id;
        Err(JournalError::Stopped)
    }

    /// Reconcile from a trusted authenticated exchange snapshot, never from
    /// a client timeout alone. The exchange ID is immutable after first set.
    pub fn reconcile(
        &mut self,
        id: &str,
        exchange_id: &str,
        state: State,
        cumulative_filled: u128,
    ) -> Result<(), JournalError> {
        if exchange_id.is_empty() || exchange_id.len() > 128 {
            return Err(JournalError::InvalidInput);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let raw: Option<(String, Option<String>, String, String)> = tx
            .query_row(
                "SELECT state,exchange_order_id,base_units,filled_units FROM order_journal
                 WHERE client_order_id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        let Some((previous, existing_id, base, previous_filled)) = raw else {
            return Err(JournalError::NotFound);
        };
        let previous = State::parse(&previous)?;
        let base: u128 = base.parse().map_err(|_| JournalError::Inconsistent)?;
        let previous_filled: u128 = previous_filled
            .parse()
            .map_err(|_| JournalError::Inconsistent)?;
        if matches!(previous, State::Filled | State::Rejected | State::Cancelled)
            || matches!(state, State::Reserved | State::Submitting | State::Unknown)
            || existing_id.as_deref().is_some_and(|known| known != exchange_id)
            || cumulative_filled < previous_filled
            || cumulative_filled > base
            || (state == State::Filled && cumulative_filled != base)
            || (state == State::Rejected && cumulative_filled != 0)
            || (state == State::Open && cumulative_filled != 0)
            || (state == State::PartiallyFilled && (cumulative_filled == 0 || cumulative_filled == base))
        {
            return Err(JournalError::Inconsistent);
        }
        // No exchange transition can be interpreted as proof that an
        // unsubmitted reservation was transmitted to the exchange.
        if previous == State::Reserved {
            return Err(JournalError::InvalidTransition);
        }
        tx.execute(
            "UPDATE order_journal SET state=?2,exchange_order_id=?3,filled_units=?4
             WHERE client_order_id=?1",
            params![id, state.as_str(), exchange_id, cumulative_filled.to_string()],
        )?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn restart_is_stopped_and_never_erases_existing_history() {
        let d = tempdir().unwrap();
        let file = d.path().join("orders.db");
        {
            let mut journal = OrderJournal::open(&file).unwrap();
            assert!(journal.stopped().unwrap());
            assert!(matches!(
                journal.reserve("client-1", 100_000_000),
                Err(JournalError::Stopped)
            ));
            journal.emergency_stop().unwrap();
        }
        let reopened = OrderJournal::open(&file).unwrap();
        assert!(reopened.stopped().unwrap());
        assert!(reopened.get("client-1").unwrap().is_none());
    }

    #[test]
    fn offline_journal_denies_submission_without_trading_activation() {
        let d = tempdir().unwrap();
        let mut journal = OrderJournal::open(&d.path().join("orders.db")).unwrap();
        assert!(matches!(
            journal.mark_submitting("anything"),
            Err(JournalError::Stopped)
        ));
        assert!(matches!(
            journal.mark_unknown("anything"),
            Err(JournalError::InvalidTransition)
        ));
    }

    #[test]
    fn rejects_malformed_client_order_ids() {
        let d = tempdir().unwrap();
        let mut journal = OrderJournal::open(&d.path().join("orders.db")).unwrap();
        assert!(matches!(
            journal.reserve("../../unsafe", 1),
            Err(JournalError::InvalidInput)
        ));
        assert!(matches!(
            journal.reserve("client-2", 0),
            Err(JournalError::InvalidInput)
        ));
    }
}
