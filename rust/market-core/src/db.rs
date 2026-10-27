//! Experimental isolated SQLite storage. This is not connected to the running bot.
//!
//! A database must be created at a new path. Existing databases are never opened
//! or migrated by this API. WAL permits concurrent readers, not concurrent writers.
use std::fmt;
use std::fs::OpenOptions;
use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, SecondsFormat, Utc};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OpenFlags};

const POOL_SIZE: u32 = 4;
const POOL_WAIT: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub enum StoreError {
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    Pool(r2d2::Error),
    WalUnavailable,
    InvalidTick,
    InvalidTimeout,
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "database file error: {error}"),
            Self::Sqlite(error) => write!(f, "SQLite error: {error}"),
            Self::Pool(error) => write!(f, "connection pool error: {error}"),
            Self::WalUnavailable => write!(f, "SQLite did not enable WAL"),
            Self::InvalidTick => write!(f, "invalid market tick"),
            Self::InvalidTimeout => write!(f, "invalid SQLite busy timeout"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<std::io::Error> for StoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}
impl From<r2d2::Error> for StoreError {
    fn from(error: r2d2::Error) -> Self {
        Self::Pool(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symbol {
    BtcUsdt,
    EthUsdt,
    SolUsdt,
}
impl Symbol {
    fn as_str(self) -> &'static str {
        match self {
            Self::BtcUsdt => "BTC/USDT",
            Self::EthUsdt => "ETH/USDT",
            Self::SolUsdt => "SOL/USDT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exchange {
    Kraken,
    Binance,
}
impl Exchange {
    fn as_str(self) -> &'static str {
        match self {
            Self::Kraken => "Kraken",
            Self::Binance => "Binance",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Tick {
    pub timestamp: DateTime<Utc>,
    pub symbol: Symbol,
    pub exchange: Exchange,
    pub price: f64,
}

/// Clone shares a bounded pool. All clones must be dropped before removing a DB.
#[derive(Clone)]
pub struct MarketStore {
    pool: Pool<SqliteConnectionManager>,
}

impl MarketStore {
    /// Create a new isolated database. Never overwrites an existing path.
    /// If initialization fails, the newly-created file may remain for inspection.
    pub fn create_new(path: &Path, busy_timeout: Duration) -> Result<Self, StoreError> {
        if busy_timeout.as_millis() > i32::MAX as u128 {
            return Err(StoreError::InvalidTimeout);
        }
        let file: std::fs::File = OpenOptions::new().write(true).create_new(true).open(path)?;
        drop(file);
        let flags: OpenFlags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE;
        let connection: Connection = Connection::open_with_flags(path, flags)?;
        connection.busy_timeout(busy_timeout)?;
        let journal_mode: String =
            connection.query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
        if !journal_mode.eq_ignore_ascii_case("wal") {
            return Err(StoreError::WalUnavailable);
        }
        connection.execute_batch(
            "CREATE TABLE market_ticks (id INTEGER PRIMARY KEY, timestamp_utc TEXT NOT NULL, symbol TEXT NOT NULL, exchange TEXT NOT NULL, price REAL NOT NULL CHECK(price > 0));\n             CREATE INDEX idx_market_ticks_lookup ON market_ticks(symbol, exchange, timestamp_utc);\n             PRAGMA user_version=1;"
        )?;
        drop(connection);
        let manager: SqliteConnectionManager = SqliteConnectionManager::file(path)
            .with_flags(flags)
            .with_init(move |connection: &mut Connection| connection.busy_timeout(busy_timeout));
        let pool: Pool<SqliteConnectionManager> = Pool::builder()
            .max_size(POOL_SIZE)
            .connection_timeout(POOL_WAIT)
            .build(manager)?;
        Ok(Self { pool })
    }

    fn connection(&self) -> Result<PooledConnection<SqliteConnectionManager>, StoreError> {
        self.pool.get().map_err(StoreError::Pool)
    }

    /// Each tick is one atomic INSERT. Persistent contention is returned, not hidden.
    pub fn record_tick(&self, tick: Tick) -> Result<(), StoreError> {
        if !tick.price.is_finite() || tick.price <= 0.0_f64 {
            return Err(StoreError::InvalidTick);
        }
        let timestamp: String = tick.timestamp.to_rfc3339_opts(SecondsFormat::Millis, true);
        let connection: PooledConnection<SqliteConnectionManager> = self.connection()?;
        connection.execute(
            "INSERT INTO market_ticks(timestamp_utc, symbol, exchange, price) VALUES (?1, ?2, ?3, ?4)",
            params![timestamp, tick.symbol.as_str(), tick.exchange.as_str(), tick.price],
        )?;
        Ok(())
    }

    pub fn tick_count(&self) -> Result<i64, StoreError> {
        let connection: PooledConnection<SqliteConnectionManager> = self.connection()?;
        Ok(connection.query_row("SELECT COUNT(*) FROM market_ticks", [], |row| row.get(0))?)
    }
}

#[cfg(test)]
mod tests {
    use super::{Exchange, MarketStore, StoreError, Symbol, Tick};
    use chrono::{DateTime, Utc};
    use rusqlite::{Error, ErrorCode};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;

    fn tick(price: f64) -> Tick {
        let timestamp: DateTime<Utc> =
            DateTime::from_timestamp(1_700_000_000_i64, 0_u32).expect("valid fixture");
        Tick {
            timestamp,
            symbol: Symbol::BtcUsdt,
            exchange: Exchange::Kraken,
            price,
        }
    }

    #[test]
    fn pooled_connections_have_wal_and_configured_timeout() {
        let directory: tempfile::TempDir = tempfile::tempdir().expect("temporary directory");
        let path: PathBuf = directory.path().join("settings.sqlite3");
        let store: MarketStore =
            MarketStore::create_new(&path, Duration::from_millis(750)).expect("new database");
        let first = store.connection().expect("first connection");
        let second = store.connection().expect("second connection");
        for connection in [&first, &second] {
            let mode: String = connection
                .query_row("PRAGMA journal_mode", [], |row| row.get(0))
                .expect("journal mode");
            let timeout: i64 = connection
                .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
                .expect("busy timeout");
            assert_eq!(mode.to_ascii_lowercase(), "wal");
            assert_eq!(timeout, 750_i64);
        }
        drop(first);
        drop(second);
        drop(store);
        directory.close().expect("connections released");
    }

    #[test]
    fn multiple_writers_persist_all_ticks_and_release_files() {
        let directory: tempfile::TempDir = tempfile::tempdir().expect("temporary directory");
        let path: PathBuf = directory.path().join("isolated.sqlite3");
        let store: MarketStore =
            MarketStore::create_new(&path, Duration::from_secs(5)).expect("new database");
        let barrier: Arc<Barrier> = Arc::new(Barrier::new(4));
        thread::scope(|scope| {
            let mut handles = Vec::new();
            for worker in 0_u32..4_u32 {
                let store: MarketStore = store.clone();
                let barrier: Arc<Barrier> = Arc::clone(&barrier);
                handles.push(scope.spawn(move || -> Result<(), StoreError> {
                    barrier.wait();
                    for item in 0_u32..25_u32 {
                        store.record_tick(tick(f64::from(worker * 25_u32 + item + 1_u32)))?;
                    }
                    Ok(())
                }));
            }
            for handle in handles {
                handle
                    .join()
                    .expect("worker must not panic")
                    .expect("all writes succeed");
            }
        });
        assert_eq!(store.tick_count().expect("read count"), 100_i64);
        drop(store);
        std::fs::remove_file(&path).expect("all connections must release the database");
        directory
            .close()
            .expect("temporary directory must be removable");
    }

    #[test]
    fn held_write_lock_returns_busy_then_recovers_after_release() {
        let directory: tempfile::TempDir = tempfile::tempdir().expect("temporary directory");
        let path: PathBuf = directory.path().join("locked.sqlite3");
        let store: MarketStore =
            MarketStore::create_new(&path, Duration::from_millis(50)).expect("new database");
        let blocker = store.connection().expect("lock connection");
        blocker
            .execute_batch("BEGIN IMMEDIATE")
            .expect("acquire lock");
        let result: Result<(), StoreError> = store.record_tick(tick(1.0_f64));
        match result {
            Err(StoreError::Sqlite(Error::SqliteFailure(error, _))) => {
                assert!(matches!(
                    error.code,
                    ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked
                ));
            }
            other => panic!("expected a SQLite contention error, got {other:?}"),
        }
        assert_eq!(store.tick_count().expect("WAL reader"), 0_i64);
        blocker.execute_batch("ROLLBACK").expect("release lock");
        drop(blocker);
        store
            .record_tick(tick(2.0_f64))
            .expect("write after release");
        assert_eq!(store.tick_count().expect("count"), 1_i64);
        drop(store);
        directory.close().expect("connections released");
    }

    #[test]
    fn existing_file_and_invalid_inputs_are_not_modified() {
        let directory: tempfile::TempDir = tempfile::tempdir().expect("temporary directory");
        let existing: PathBuf = directory.path().join("existing.sqlite3");
        std::fs::write(&existing, b"preserve me").expect("fixture");
        assert!(matches!(
            MarketStore::create_new(&existing, Duration::from_secs(1)),
            Err(StoreError::Io(_))
        ));
        assert_eq!(
            std::fs::read(&existing).expect("read fixture"),
            b"preserve me"
        );
        let invalid: &Path = &directory.path().join("invalid.sqlite3");
        assert!(matches!(
            MarketStore::create_new(invalid, Duration::from_secs(u64::MAX)),
            Err(StoreError::InvalidTimeout)
        ));
        assert!(!invalid.exists());
        let store: MarketStore = MarketStore::create_new(
            &directory.path().join("new.sqlite3"),
            Duration::from_secs(1),
        )
        .expect("new database");
        for value in [0.0_f64, -1.0_f64, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                store.record_tick(tick(value)),
                Err(StoreError::InvalidTick)
            ));
        }
        assert_eq!(store.tick_count().expect("count"), 0_i64);
        drop(store);
        directory.close().expect("connections released");
    }
}
