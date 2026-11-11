//! Local PAPER-only control authority. No order API, network listener, or
//! permission to activate private trading. All writes are serialized.
//! Event HMACs are tamper-evident against a reader lacking the Windows key;
//! the local SQLite file is NOT immutable or protected from key compromise.
use crate::secret_store::{SecretStore, SecretStoreError, SecretValue, WindowsCredentialStore};
use rusqlite::{params, Connection, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;

const SIGNING_KEY_ID: &str = "internal_paper_audit_v1";
const REARM_COOLDOWN_SECONDS: u64 = 300;
const MAX_AUDIT_ROWS: u64 = 1_000_000;
const DOMAIN: &[u8] = b"PRAVDA-PAPER-CONTROL-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlError {
    Unavailable,
    Corrupt,
    Stopped,
    Cooldown,
    AlreadyArmed,
    Capacity,
}

impl ControlError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Unavailable => "CONTROL_STORAGE_UNAVAILABLE",
            Self::Corrupt => "CONTROL_AUDIT_INTEGRITY_FAILURE",
            Self::Stopped => "PAPER_KILL_SWITCH_ACTIVE",
            Self::Cooldown => "PAPER_REARM_COOLDOWN",
            Self::AlreadyArmed => "PAPER_ALREADY_ARMED",
            Self::Capacity => "CONTROL_AUDIT_CAPACITY_REACHED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlStatus {
    pub stopped: bool,
    pub audit_records: u64,
}

/// A hardware-backed or tamper-proof audit is NOT claimed. Credential Manager
/// protects this key under the local Windows identity, not against malware
/// executing as the same user.
pub struct PaperControl {
    connection: Mutex<VerifiedConnection>,
    key: SecretValue,
    // Bit 0 indicates STOPPED; upper bits increase on each emergency request.
    stop_epoch: AtomicU64,
}

#[derive(Clone, Copy)]
struct Verified {
    seq: u64,
    previous: [u8; 32],
    armed: bool,
    pending_intent: bool,
    last_emergency_at: Option<u64>,
}

/// Startup verifies the full journal once. The lifetime-exclusive SQLite
/// connection then owns its authenticated tail, updated only after commit.
/// This avoids the O(N²) replay of all historical HMACs during trading.
struct VerifiedConnection {
    connection: Connection,
    verified: Verified,
}

fn now_seconds() -> Result<u64, ControlError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| ControlError::Unavailable)
}

/// Standard HMAC-SHA256 over a domain-separated, unambiguous byte encoding.
fn hmac_sha256(key: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut block = [0_u8; 64];
    if key.len() > block.len() {
        block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let mut inner_key = block;
    let mut outer_key = block;
    for byte in &mut inner_key {
        *byte ^= 0x36;
    }
    for byte in &mut outer_key {
        *byte ^= 0x5c;
    }
    let mut inner = Sha256::new();
    inner.update(inner_key);
    inner.update(payload);
    let inner_digest = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(outer_key);
    outer.update(inner_digest);
    outer.finalize().into()
}

fn event_mac(
    key: &[u8],
    seq: u64,
    when: u64,
    action: &'static str,
    previous: &[u8; 32],
) -> [u8; 32] {
    let mut bytes = Vec::with_capacity(DOMAIN.len() + 8 + 8 + 32 + action.len() + 8);
    bytes.extend_from_slice(DOMAIN);
    bytes.extend_from_slice(&seq.to_be_bytes());
    bytes.extend_from_slice(&when.to_be_bytes());
    bytes.extend_from_slice(previous);
    bytes.extend_from_slice(&(action.len() as u64).to_be_bytes());
    bytes.extend_from_slice(action.as_bytes());
    hmac_sha256(key, &bytes)
}

fn action_from_storage(action: &str) -> Option<&'static str> {
    match action {
        "INITIAL_STOP" => Some("INITIAL_STOP"),
        "STARTUP_STOP" => Some("STARTUP_STOP"),
        "EMERGENCY_STOP" => Some("EMERGENCY_STOP"),
        "REARM_PAPER" => Some("REARM_PAPER"),
        "PAPER_INTENT" => Some("PAPER_INTENT"),
        "PAPER_ACCEPTED" => Some("PAPER_ACCEPTED"),
        "PAPER_REFUSED" => Some("PAPER_REFUSED"),
        _ => None,
    }
}

fn verify(connection: &Connection, key: &[u8]) -> Result<Verified, ControlError> {
    let mut stmt = connection
        .prepare(
            "SELECT seq, at_sec, action, previous_mac, mac FROM paper_control_log ORDER BY seq",
        )
        .map_err(|_| ControlError::Unavailable)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Vec<u8>>(3)?,
                row.get::<_, Vec<u8>>(4)?,
            ))
        })
        .map_err(|_| ControlError::Unavailable)?;
    let mut state = Verified {
        seq: 0,
        previous: [0; 32],
        armed: false,
        pending_intent: false,
        last_emergency_at: None,
    };
    let mut pending_intent = false;
    for record in rows {
        let (raw_seq, raw_at, action, previous, supplied) =
            record.map_err(|_| ControlError::Unavailable)?;
        let seq = u64::try_from(raw_seq).map_err(|_| ControlError::Corrupt)?;
        let when = u64::try_from(raw_at).map_err(|_| ControlError::Corrupt)?;
        let code = action_from_storage(&action).ok_or(ControlError::Corrupt)?;
        if seq != state.seq + 1
            || seq > MAX_AUDIT_ROWS
            || previous.len() != 32
            || supplied.len() != 32
            || previous.as_slice().ct_eq(&state.previous).unwrap_u8() != 1
            || supplied
                .as_slice()
                .ct_eq(&event_mac(key, seq, when, code, &state.previous))
                .unwrap_u8()
                != 1
        {
            return Err(ControlError::Corrupt);
        }
        if seq == 1 && code != "INITIAL_STOP" {
            return Err(ControlError::Corrupt);
        }
        match code {
            "INITIAL_STOP" | "STARTUP_STOP" | "EMERGENCY_STOP" => {
                state.armed = false;
                pending_intent = false;
                if code == "EMERGENCY_STOP" {
                    state.last_emergency_at = Some(when);
                }
            }
            "REARM_PAPER" => {
                if state.armed || pending_intent {
                    return Err(ControlError::Corrupt);
                }
                state.armed = true;
            }
            "PAPER_INTENT" => {
                if !state.armed || pending_intent {
                    return Err(ControlError::Corrupt);
                }
                pending_intent = true;
            }
            "PAPER_ACCEPTED" | "PAPER_REFUSED" => {
                if !pending_intent || !state.armed {
                    return Err(ControlError::Corrupt);
                }
                pending_intent = false;
            }
            _ => return Err(ControlError::Corrupt),
        }
        state.previous.copy_from_slice(&supplied);
        state.seq = seq;
    }
    // An incomplete trade is an unresolved state, not an implicit refusal.
    // Preserve it so REARM_PAPER cannot corrupt the journal by appending
    // after PAPER_INTENT without a terminal event or explicit STOP.
    state.pending_intent = pending_intent;
    if pending_intent {
        state.armed = false;
    }
    Ok(state)
}

fn append(
    ledger: &mut VerifiedConnection,
    key: &[u8],
    action: &'static str,
) -> Result<(), ControlError> {
    let previous = ledger.verified;
    // Validate state-machine transitions against the already authenticated
    // in-memory tail before starting the transaction. There is only one
    // connection/owner; every write passes through this mutex.
    match action {
        "INITIAL_STOP" if previous.seq == 0 => {}
        "STARTUP_STOP" | "EMERGENCY_STOP" => {}
        "REARM_PAPER" if !previous.armed && !previous.pending_intent => {}
        "PAPER_INTENT" if previous.armed && !previous.pending_intent => {}
        "PAPER_ACCEPTED" | "PAPER_REFUSED" if previous.armed && previous.pending_intent => {}
        _ => return Err(ControlError::Corrupt),
    }
    let seq = previous.seq.checked_add(1).ok_or(ControlError::Capacity)?;
    if seq > MAX_AUDIT_ROWS {
        return Err(ControlError::Capacity);
    }
    let when = now_seconds()?;
    let tag = event_mac(key, seq, when, action, &previous.previous);
    let tx = ledger
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| ControlError::Unavailable)?;
    tx.execute(
        "INSERT INTO paper_control_log(seq,at_sec,action,previous_mac,mac) VALUES(?1,?2,?3,?4,?5)",
        params![
            i64::try_from(seq).map_err(|_| ControlError::Capacity)?,
            i64::try_from(when).map_err(|_| ControlError::Unavailable)?,
            action,
            &previous.previous[..],
            &tag[..]
        ],
    )
    .map_err(|_| ControlError::Unavailable)?;
    tx.commit().map_err(|_| ControlError::Unavailable)?;

    // Only a successful COMMIT advances the cached authenticated state.
    // Any write failure leaves the old tail intact and the caller fails
    // closed, without assuming the transaction was persisted.
    let state = &mut ledger.verified;
    state.seq = seq;
    state.previous = tag;
    match action {
        "INITIAL_STOP" | "STARTUP_STOP" | "EMERGENCY_STOP" => {
            state.armed = false;
            state.pending_intent = false;
            if action == "EMERGENCY_STOP" {
                state.last_emergency_at = Some(when);
            }
        }
        "REARM_PAPER" => state.armed = true,
        "PAPER_INTENT" => state.pending_intent = true,
        "PAPER_ACCEPTED" | "PAPER_REFUSED" => state.pending_intent = false,
        _ => unreachable!("previously validated state-machine action"),
    }
    Ok(())
}

impl PaperControl {
    /// Acquire the lifetime-long SQLite EXCLUSIVE lock BEFORE touching the
    /// Windows credential. Two simultaneous first launches must never race
    /// CredWriteW and permanently replace the HMAC signing key.
    fn exclusive_connection(path: &Path) -> Result<Connection, ControlError> {
        let connection = Connection::open(path).map_err(|_| ControlError::Unavailable)?;
        connection
            .execute_batch(
                "PRAGMA busy_timeout=1000;
                 PRAGMA locking_mode=EXCLUSIVE;
                 PRAGMA journal_mode=WAL;
                 PRAGMA synchronous=FULL;
                 BEGIN IMMEDIATE;
                 COMMIT;",
            )
            .map_err(|_| ControlError::Unavailable)?;
        Ok(connection)
    }

    /// Production entry point. On non-Windows platforms this refuses to
    /// generate a plaintext fallback key or silently arm the application.
    pub fn open(path: &Path) -> Result<Self, ControlError> {
        // Keep the lock open through credential read/create, journal
        // initialization, and every subsequent PAPER mutation.
        let connection = Self::exclusive_connection(path)?;
        let vault = WindowsCredentialStore;
        let key = match vault.get(SIGNING_KEY_ID) {
            Ok(existing) => existing,
            Err(SecretStoreError::NotFound) => {
                let mut bytes = vec![0_u8; 32];
                getrandom::fill(&mut bytes).map_err(|_| ControlError::Unavailable)?;
                let value = SecretValue::new(bytes);
                vault
                    .put(SIGNING_KEY_ID, &value)
                    .map_err(|_| ControlError::Unavailable)?;
                value
            }
            Err(_) => return Err(ControlError::Unavailable),
        };
        Self::open_locked(connection, key)
    }

    #[cfg(test)]
    fn open_with_key(path: &Path, key: SecretValue) -> Result<Self, ControlError> {
        let connection = Self::exclusive_connection(path)?;
        Self::open_locked(connection, key)
    }

    fn open_locked(connection: Connection, key: SecretValue) -> Result<Self, ControlError> {
        if key.expose().len() != 32 {
            return Err(ControlError::Corrupt);
        }
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS paper_control_log(
                    seq INTEGER PRIMARY KEY NOT NULL,
                    at_sec INTEGER NOT NULL,
                    action TEXT NOT NULL,
                    previous_mac BLOB NOT NULL,
                    mac BLOB NOT NULL
                 );",
            )
            .map_err(|_| ControlError::Unavailable)?;
        // Full HMAC/sequence/state-machine replay is a mandatory STARTUP
        // operation, not part of a synchronous PAPER trade.
        let state = verify(&connection, key.expose())?;
        let mut ledger = VerifiedConnection {
            connection,
            verified: state,
        };
        if state.seq == 0 {
            append(&mut ledger, key.expose(), "INITIAL_STOP")?;
        }
        // A restart must always stop, irrespective of prior arm state.
        append(&mut ledger, key.expose(), "STARTUP_STOP")?;
        Ok(Self {
            connection: Mutex::new(ledger),
            key,
            stop_epoch: AtomicU64::new(1),
        })
    }

    pub fn status(&self) -> Result<ControlStatus, ControlError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ControlError::Unavailable)?;
        let state = connection.verified;
        Ok(ControlStatus {
            stopped: self.is_stopped() || !state.armed,
            audit_records: state.seq,
        })
    }

    fn is_stopped(&self) -> bool {
        self.stop_epoch.load(Ordering::SeqCst) & 1 != 0
    }

    fn request_stop(&self) -> Result<(), ControlError> {
        self.stop_epoch
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
                value.checked_add(2).map(|next| next | 1)
            })
            .map_err(|_| ControlError::Capacity)?;
        Ok(())
    }

    pub fn emergency_stop(&self) -> Result<(), ControlError> {
        // Immediate veto must not wait for the journal mutex or for PBKDF2.
        self.request_stop()?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ControlError::Unavailable)?;
        append(&mut connection, self.key.expose(), "EMERGENCY_STOP")?;
        Ok(())
    }

    /// Caller MUST verify the password on a blocking worker and keep the
    /// authenticated Rust session lock throughout the final native mutation.
    /// The paper reset is performed only after the durable rearm succeeded.
    pub fn rearm_with_reset<T>(&self, reset: impl FnOnce() -> T) -> Result<T, ControlError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ControlError::Unavailable)?;
        let expected_epoch = self.stop_epoch.load(Ordering::SeqCst);
        let state = connection.verified;
        if state.pending_intent {
            return Err(ControlError::Stopped);
        }
        if expected_epoch & 1 == 0 || state.armed {
            return Err(ControlError::AlreadyArmed);
        }
        if let Some(when) = state.last_emergency_at {
            let since = now_seconds()?
                .checked_sub(when)
                .ok_or(ControlError::Cooldown)?;
            if since < REARM_COOLDOWN_SECONDS {
                return Err(ControlError::Cooldown);
            }
        }
        append(&mut connection, self.key.expose(), "REARM_PAPER")?;
        // A concurrently requested emergency stop increments the epoch
        // before obtaining the journal lock: CAS must never clear that stop.
        self.stop_epoch
            .compare_exchange(
                expected_epoch,
                expected_epoch & !1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .map_err(|_| ControlError::Stopped)?;
        Ok(reset())
    }

    /// Serializes stop/rearm with the existing synchronous paper mutation.
    /// Log INTENT before mutation; refusal/acceptance afterwards. If the
    /// second log fails, lock out future paper operations until investigation.
    pub fn with_paper_trade<T>(
        &self,
        trade: impl FnOnce() -> Result<T, &'static str>,
    ) -> Result<T, &'static str> {
        if self.is_stopped() {
            return Err(ControlError::Stopped.code());
        }
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ControlError::Unavailable.code())?;
        let state = connection.verified;
        if !state.armed || self.is_stopped() {
            let _ = self.request_stop();
            return Err(ControlError::Stopped.code());
        }
        if let Err(error) = append(&mut connection, self.key.expose(), "PAPER_INTENT") {
            let _ = self.request_stop();
            return Err(error.code());
        }
        let result = trade();
        let code = if result.is_ok() {
            "PAPER_ACCEPTED"
        } else {
            "PAPER_REFUSED"
        };
        if let Err(error) = append(&mut connection, self.key.expose(), code) {
            let _ = self.request_stop();
            return Err(error.code());
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn test_control(path: &Path) -> Result<PaperControl, ControlError> {
        PaperControl::open_with_key(path, SecretValue::new(vec![17_u8; 32]))
    }

    #[test]
    fn hmac_matches_a_standard_known_answer() {
        let mac = hmac_sha256(&[0x0b; 20], b"Hi There");
        let expected: [u8; 32] = [
            0xb0, 0x34, 0x4c, 0x61, 0xd8, 0xdb, 0x38, 0x53, 0x5c, 0xa8, 0xaf, 0xce, 0xaf, 0x0b,
            0xf1, 0x2b, 0x88, 0x1d, 0xc2, 0x00, 0xc9, 0x83, 0x3d, 0xa7, 0x26, 0xe9, 0x37, 0x6c,
            0x2e, 0x32, 0xcf, 0xf7,
        ];
        assert_eq!(mac, expected);
    }

    #[test]
    fn starts_stopped_and_restart_revokes_previous_arm() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("control.sqlite");
        let initial = test_control(&path).unwrap();
        assert!(initial.status().unwrap().stopped);
        assert_eq!(
            initial.with_paper_trade(|| Ok::<_, &'static str>(())),
            Err("PAPER_KILL_SWITCH_ACTIVE")
        );
        initial.rearm_with_reset(|| ()).unwrap();
        assert!(!initial.status().unwrap().stopped);
        assert_eq!(
            initial.with_paper_trade(|| Ok::<_, &'static str>(42)),
            Ok(42)
        );
        drop(initial);
        let reopened = test_control(&path).unwrap();
        assert!(reopened.status().unwrap().stopped);
        assert!(reopened.status().unwrap().audit_records >= 5);
    }

    #[test]
    fn emergency_stop_disallows_new_trades_and_immediate_rearm() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("control.sqlite");
        let control = test_control(&path).unwrap();
        control.rearm_with_reset(|| ()).unwrap();
        control.emergency_stop().unwrap();
        assert_eq!(control.rearm_with_reset(|| ()), Err(ControlError::Cooldown));
        assert_eq!(
            control.with_paper_trade(|| Ok::<_, &'static str>(1)),
            Err("PAPER_KILL_SWITCH_ACTIVE")
        );
    }

    #[test]
    fn tampering_with_record_blocks_reopening() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("control.sqlite");
        let control = test_control(&path).unwrap();
        drop(control);
        let connection = Connection::open(&path).unwrap();
        connection
            .execute(
                "UPDATE paper_control_log SET action='REARM_PAPER' WHERE seq=1",
                [],
            )
            .unwrap();
        drop(connection);
        assert!(matches!(test_control(&path), Err(ControlError::Corrupt)));
    }

    #[test]
    fn wrong_key_cannot_validate_previous_mac_chain() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("control.sqlite");
        drop(test_control(&path).unwrap());
        let wrong = PaperControl::open_with_key(&path, SecretValue::new(vec![18; 32]));
        assert!(matches!(wrong, Err(ControlError::Corrupt)));
    }

    #[test]
    fn failed_paper_intent_is_logged_and_does_not_arm() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("control.sqlite");
        let control = test_control(&path).unwrap();
        control.rearm_with_reset(|| ()).unwrap();
        assert_eq!(
            control.with_paper_trade(|| Err::<(), _>("INVALID_PAPER_QUANTITY")),
            Err("INVALID_PAPER_QUANTITY")
        );
        let status = control.status().unwrap();
        assert!(!status.stopped);
        assert_eq!(status.audit_records, 5);
    }

    #[test]
    fn corrupted_bytes_are_not_silently_reinitialized() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("control.sqlite");
        fs::write(&path, b"not a database").unwrap();
        assert!(test_control(&path).is_err());
    }
    #[test]
    fn duplicate_arm_never_resets_portfolio_or_corrupts_journal() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let directory = tempdir().unwrap();
        let path = directory.path().join("control.sqlite");
        let control = test_control(&path).unwrap();
        let resets = AtomicUsize::new(0);
        control
            .rearm_with_reset(|| resets.fetch_add(1, Ordering::SeqCst))
            .unwrap();
        assert_eq!(resets.load(Ordering::SeqCst), 1);
        let before = control.status().unwrap().audit_records;
        assert_eq!(
            control.rearm_with_reset(|| resets.fetch_add(1, Ordering::SeqCst)),
            Err(ControlError::AlreadyArmed)
        );
        assert_eq!(resets.load(Ordering::SeqCst), 1);
        assert_eq!(control.status().unwrap().audit_records, before);
        drop(control);
        assert!(test_control(&path).unwrap().status().unwrap().stopped);
    }

    #[test]
    fn interrupted_stop_append_never_commits_a_second_arm() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let directory = tempdir().unwrap();
        let path = directory.path().join("control.sqlite");
        let control = test_control(&path).unwrap();
        control.rearm_with_reset(|| ()).unwrap();
        // Reproduce a stop request that reached memory but whose journal
        // append failed. The persisted journal still contains REARM_PAPER.
        control.request_stop().unwrap();
        let before = control.status().unwrap().audit_records;
        let resets = AtomicUsize::new(0);
        assert_eq!(
            control.rearm_with_reset(|| resets.fetch_add(1, Ordering::SeqCst)),
            Err(ControlError::AlreadyArmed)
        );
        assert_eq!(resets.load(Ordering::SeqCst), 0);
        assert_eq!(control.status().unwrap().audit_records, before);
        // A new explicit emergency stop records the missing stop transition.
        control.emergency_stop().unwrap();
        assert_eq!(control.rearm_with_reset(|| ()), Err(ControlError::Cooldown));
        drop(control);
        assert!(test_control(&path).unwrap().status().unwrap().stopped);
    }

    #[test]
    fn rejected_rearm_during_cooldown_never_resets_portfolio() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let directory = tempdir().unwrap();
        let path = directory.path().join("control.sqlite");
        let control = test_control(&path).unwrap();
        control.rearm_with_reset(|| ()).unwrap();
        control.emergency_stop().unwrap();
        let resets = AtomicUsize::new(0);
        assert_eq!(
            control.rearm_with_reset(|| resets.fetch_add(1, Ordering::SeqCst)),
            Err(ControlError::Cooldown)
        );
        assert_eq!(resets.load(Ordering::SeqCst), 0);
        assert!(control.status().unwrap().stopped);
    }

    #[test]
    fn second_instance_cannot_reset_a_live_paper_intent() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("control.sqlite");
        let first = test_control(&path).unwrap();
        first.rearm_with_reset(|| ()).unwrap();
        let second = test_control(&path);
        assert!(
            matches!(second, Err(ControlError::Unavailable)),
            "another SQLite connection must not acquire journal ownership"
        );
        assert_eq!(first.with_paper_trade(|| Ok::<_, &'static str>(7)), Ok(7));
        let status = first.status().unwrap();
        assert!(!status.stopped);
        drop(first);
        let reopened = test_control(&path).unwrap();
        assert!(reopened.status().unwrap().stopped);
    }
    #[test]
    fn unfinalized_trade_intent_requires_stop_before_rearming() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let directory = tempdir().unwrap();
        let path = directory.path().join("control.sqlite");
        let control = test_control(&path).unwrap();
        control.rearm_with_reset(|| ()).unwrap();
        // Simulate an accepted PAPER_INTENT followed by a journal-write
        // failure before PAPER_ACCEPTED/PAPER_REFUSED is written.
        {
            let mut connection = control.connection.lock().unwrap();
            append(&mut connection, control.key.expose(), "PAPER_INTENT").unwrap();
        }
        control.request_stop().unwrap();
        let before = control.status().unwrap().audit_records;
        let resets = AtomicUsize::new(0);
        assert_eq!(
            control.rearm_with_reset(|| resets.fetch_add(1, Ordering::SeqCst)),
            Err(ControlError::Stopped)
        );
        assert_eq!(resets.load(Ordering::SeqCst), 0);
        assert_eq!(control.status().unwrap().audit_records, before);
        control.emergency_stop().unwrap();
        assert_eq!(control.rearm_with_reset(|| ()), Err(ControlError::Cooldown));
        drop(control);
        assert!(test_control(&path).unwrap().status().unwrap().stopped);
    }

    #[test]
    fn cached_authenticated_tail_advances_without_replaying_history() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("control.sqlite");
        let control = test_control(&path).unwrap();
        control.rearm_with_reset(|| ()).unwrap();
        let mut last_count = control.status().unwrap().audit_records;
        for _ in 0..64 {
            assert_eq!(control.with_paper_trade(|| Ok::<_, &'static str>(1)), Ok(1));
            let current = control.status().unwrap().audit_records;
            assert_eq!(current, last_count + 2);
            last_count = current;
        }
        // Full verification remains mandatory on a *new* process startup.
        drop(control);
        let reopened = test_control(&path).unwrap();
        assert!(reopened.status().unwrap().stopped);
        assert_eq!(reopened.status().unwrap().audit_records, last_count + 1);
    }

    #[test]
    fn journal_lock_is_acquired_before_initializing_signing_key() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("control.sqlite");
        // This simulates a first launch holding the control lock but not yet
        // having read or generated an audit signing key.
        let first = PaperControl::exclusive_connection(&path).unwrap();
        assert!(matches!(
            PaperControl::exclusive_connection(&path),
            Err(ControlError::Unavailable)
        ));
        drop(first);
        assert!(test_control(&path).unwrap().status().unwrap().stopped);
    }
}
