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
             PRAGMA synchronous=FULL;
             PRAGMA foreign_keys=ON;
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
             );
             UPDATE order_journal_control SET stopped=1 WHERE id=1;",
        )?;
        // Fail-closed across process restarts. A reopened database is stopped
        // until an explicit, separately authorized future activation procedure.
        // SEC: after an unacknowledged request, a restart must never mistake
        // an in-flight submission for permission to retry.
        // SEC: Persist STOPPED before potentially fallible legacy migrations.
        // A duplicate venue identity must never roll back the kill switch.
        connection.execute(
            "UPDATE order_journal_control SET stopped=1 WHERE id=1",
            [],
        )?;
        // SEC: Validate original historical lifecycle records before any
        // normalization; malformed 'submitting' rows must never be hidden.
        Self::verify_existing_rows(&connection)?;
        connection.execute_batch(
            "BEGIN IMMEDIATE;
             UPDATE order_journal SET state='unknown' WHERE state='submitting';
             CREATE UNIQUE INDEX IF NOT EXISTS ux_order_journal_venue_identity
               ON order_journal(exchange_order_id)
               WHERE exchange_order_id IS NOT NULL;
             COMMIT;",
        )?;
        // SEC: Revalidate the migrated state and venue uniqueness.
        Self::verify_existing_rows(&connection)?;
        Ok(Self { connection })
    }

    fn verify_existing_rows(connection: &Connection) -> Result<(), JournalError> {
        let integrity: String = connection.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
        if integrity != "ok" {
            return Err(JournalError::Inconsistent);
        }
        let mut query = connection
            .prepare("SELECT state,exchange_order_id,base_units,filled_units FROM order_journal")?;
        let records = query.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        for record in records {
            let (raw_state, venue, raw_base, raw_filled) = record?;
            let state = State::parse(&raw_state)?;
            let base: u128 = raw_base.parse().map_err(|_| JournalError::Inconsistent)?;
            let filled: u128 = raw_filled.parse().map_err(|_| JournalError::Inconsistent)?;
            if base == 0
                || filled > base
                || venue.as_deref().is_some_and(str::is_empty)
                || (matches!(state, State::Reserved | State::Submitting) && venue.is_some())
                || (matches!(
                    state,
                    State::Open
                        | State::PartiallyFilled
                        | State::Filled
                        | State::Cancelled
                        | State::Rejected
                ) && venue.is_none())
                || (state == State::Reserved && filled != 0)
                || (state == State::Open && filled != 0)
                || (state == State::PartiallyFilled && (filled == 0 || filled == base))
                || (state == State::Filled && filled != base)
                || (state == State::Rejected && filled != 0)
            {
                return Err(JournalError::Inconsistent);
            }
        }
        Ok(())
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
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("UPDATE order_journal_control SET stopped=1 WHERE id=1", [])?;
        tx.commit()?;
        Ok(())
    }

    /// Enable reservations only for offline unit tests. This method is not
    /// compiled in production; trading and submission remain unavailable.
    #[cfg(test)]
    fn test_allow_offline_reservation(&mut self) -> Result<(), JournalError> {
        self.connection
            .execute("UPDATE order_journal_control SET stopped=0 WHERE id=1", [])?;
        Ok(())
    }

    /// Deliberately no public enable method: readiness, authorization and
    /// broker risk checks must be independently designed and reviewed.
    pub fn reserve(&mut self, id: &str, base_units: u128) -> Result<(), JournalError> {
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
            || existing_id
                .as_deref()
                .is_some_and(|known| known != exchange_id)
            || cumulative_filled < previous_filled
            || cumulative_filled > base
            || (state == State::Filled && cumulative_filled != base)
            || (state == State::Rejected && cumulative_filled != 0)
            || (state == State::Open && cumulative_filled != 0)
            || (state == State::PartiallyFilled
                && (cumulative_filled == 0 || cumulative_filled == base))
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
            params![
                id,
                state.as_str(),
                exchange_id,
                cumulative_filled.to_string()
            ],
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
    fn corrupted_order_quantities_refuse_startup_even_with_valid_sqlite() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("orders.db");
        {
            let mut journal = OrderJournal::open(&file).unwrap();
            journal.test_allow_offline_reservation().unwrap();
            journal.reserve("safe-id", 100).unwrap();
        }
        {
            let connection = Connection::open(&file).unwrap();
            connection
                .execute(
                    "UPDATE order_journal SET filled_units='101' WHERE client_order_id='safe-id'",
                    [],
                )
                .unwrap();
        }
        assert!(matches!(
            OrderJournal::open(&file),
            Err(JournalError::Inconsistent)
        ));
        let conn = Connection::open(&file).unwrap();
        let stopped: bool = conn
            .query_row(
                "SELECT stopped FROM order_journal_control WHERE id=1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(stopped);
    }

    #[test]
    fn one_venue_order_can_never_be_bound_to_two_clients() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("orders.db");
        let mut journal = OrderJournal::open(&file).unwrap();
        journal.test_allow_offline_reservation().unwrap();
        for id in ["client-a", "client-b"] {
            journal.reserve(id, 100).unwrap();
            journal
                .connection
                .execute(
                    "UPDATE order_journal SET state='unknown' WHERE client_order_id=?1",
                    [id],
                )
                .unwrap();
        }
        journal
            .reconcile("client-a", "venue-123", State::Open, 0)
            .unwrap();
        assert!(journal
            .reconcile("client-b", "venue-123", State::Open, 0)
            .is_err());
        assert!(journal
            .get("client-b")
            .unwrap()
            .unwrap()
            .exchange_id
            .is_none());
    }

    #[test]
    fn malformed_submitting_record_is_rejected_before_normalization() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("orders.db");
        {
            let mut journal = OrderJournal::open(&file).unwrap();
            journal.test_allow_offline_reservation().unwrap();
            journal.reserve("pending-legacy", 100).unwrap();
        }
        {
            let connection = Connection::open(&file).unwrap();
            connection
                .execute(
                    "UPDATE order_journal SET state='submitting', exchange_order_id='venue-tampered' WHERE client_order_id='pending-legacy'",
                    [],
                )
                .unwrap();
        }
        assert!(matches!(OrderJournal::open(&file), Err(JournalError::Inconsistent)));
        let connection = Connection::open(&file).unwrap();
        let state: String = connection
            .query_row(
                "SELECT state FROM order_journal WHERE client_order_id='pending-legacy'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(state, "submitting");
        let stopped: bool = connection
            .query_row(
                "SELECT stopped FROM order_journal_control WHERE id=1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(stopped);
    }

    #[test]
    fn index_migration_failure_preserves_emergency_stop() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("orders.db");
        {
            let connection = Connection::open(&file).unwrap();
            connection.execute_batch(
                "CREATE TABLE order_journal_control(id INTEGER PRIMARY KEY, stopped INTEGER NOT NULL);
                 INSERT INTO order_journal_control VALUES(1,0);
                 CREATE TABLE order_journal(
                   client_order_id TEXT PRIMARY KEY,
                   state TEXT NOT NULL,
                   exchange_order_id TEXT,
                   base_units TEXT NOT NULL,
                   filled_units TEXT NOT NULL
                 );
                 INSERT INTO order_journal VALUES('client-a','open','collision','100','0');
                 INSERT INTO order_journal VALUES('client-b','open','collision','100','0');",
            ).unwrap();
        }
        assert!(OrderJournal::open(&file).is_err());
        // SEC: Verify the kill switch is DURABLE even when index creation
        // fails and the application refuses to open the corrupt journal.
        let connection = Connection::open(&file).unwrap();
        let stopped: bool = connection
            .query_row(
                "SELECT stopped FROM order_journal_control WHERE id=1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(stopped);
        drop(connection);
        assert!(OrderJournal::open(&file).is_err());
    }

    #[test]
    fn old_schema_with_duplicate_venue_ids_fails_migration_closed() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("orders.db");
        {
            let connection = Connection::open(&file).unwrap();
            connection.execute_batch(
                "CREATE TABLE order_journal_control(id INTEGER PRIMARY KEY, stopped INTEGER NOT NULL);
                 INSERT INTO order_journal_control VALUES(1,0);
                 CREATE TABLE order_journal(
                   client_order_id TEXT PRIMARY KEY,
                   state TEXT NOT NULL,
                   exchange_order_id TEXT,
                   base_units TEXT NOT NULL,
                   filled_units TEXT NOT NULL
                 );
                 INSERT INTO order_journal VALUES('client-a','open','same-id','100','0');
                 INSERT INTO order_journal VALUES('client-b','open','same-id','100','0');",
            ).unwrap();
        }
        assert!(OrderJournal::open(&file).is_err());
    }

    #[test]
    fn crash_during_submission_becomes_unknown_before_any_new_request() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("orders.db");
        {
            let mut journal = OrderJournal::open(&file).unwrap();
            journal.test_allow_offline_reservation().unwrap();
            journal.reserve("crashed-send", 100).unwrap();
            journal
                .connection
                .execute(
                    "UPDATE order_journal SET state='submitting' WHERE client_order_id=?1",
                    ["crashed-send"],
                )
                .unwrap();
        }
        let mut recovered = OrderJournal::open(&file).unwrap();
        assert!(recovered.stopped().unwrap());
        assert_eq!(
            recovered.get("crashed-send").unwrap().unwrap().state,
            State::Unknown
        );
        assert!(matches!(
            recovered.mark_submitting("crashed-send"),
            Err(JournalError::Stopped)
        ));
        recovered.test_allow_offline_reservation().unwrap();
        assert!(matches!(
            recovered.reserve("crashed-send", 100),
            Err(JournalError::Duplicate)
        ));
    }

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
    fn duplicate_reservations_survive_process_restart() {
        let d = tempdir().unwrap();
        let path = d.path().join("orders.db");
        {
            let mut journal = OrderJournal::open(&path).unwrap();
            journal.test_allow_offline_reservation().unwrap();
            journal.reserve("fixed-id-1", 100_000_000).unwrap();
            assert!(matches!(
                journal.reserve("fixed-id-1", 100_000_000),
                Err(JournalError::Duplicate)
            ));
        }
        let mut after_restart = OrderJournal::open(&path).unwrap();
        assert!(after_restart.stopped().unwrap());
        let record = after_restart.get("fixed-id-1").unwrap().unwrap();
        assert_eq!(record.state, State::Reserved);
        assert_eq!(record.base_units, 100_000_000);
        assert!(matches!(
            after_restart.reserve("fixed-id-1", 100_000_000),
            Err(JournalError::Stopped)
        ));
        after_restart.test_allow_offline_reservation().unwrap();
        assert!(matches!(
            after_restart.reserve("fixed-id-1", 100_000_000),
            Err(JournalError::Duplicate)
        ));
    }

    #[test]
    fn uncertainty_is_persistent_and_cannot_be_resubmitted() {
        let d = tempdir().unwrap();
        let file = d.path().join("orders.db");
        {
            let mut journal = OrderJournal::open(&file).unwrap();
            journal.test_allow_offline_reservation().unwrap();
            journal.reserve("fixed-id-2", 100).unwrap();
            // This SQL transition models an exchange send in a test fixture.
            // No network sender is compiled or enabled in the production API.
            journal
                .connection
                .execute(
                    "UPDATE order_journal SET state='submitting' WHERE client_order_id=?1",
                    ["fixed-id-2"],
                )
                .unwrap();
            journal.mark_unknown("fixed-id-2").unwrap();
        }
        let mut journal = OrderJournal::open(&file).unwrap();
        let record = journal.get("fixed-id-2").unwrap().unwrap();
        assert_eq!(record.state, State::Unknown);
        assert!(journal.stopped().unwrap());
        assert!(matches!(
            journal.mark_submitting("fixed-id-2"),
            Err(JournalError::Stopped)
        ));
        journal
            .reconcile("fixed-id-2", "exchange-a", State::PartiallyFilled, 40)
            .unwrap();
        assert!(matches!(
            journal.reconcile("fixed-id-2", "exchange-b", State::Filled, 100),
            Err(JournalError::Inconsistent)
        ));
        assert!(matches!(
            journal.reconcile("fixed-id-2", "exchange-a", State::PartiallyFilled, 30),
            Err(JournalError::Inconsistent)
        ));
        journal
            .reconcile("fixed-id-2", "exchange-a", State::Filled, 100)
            .unwrap();
        assert_eq!(
            journal.get("fixed-id-2").unwrap().unwrap().filled_units,
            100
        );
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
