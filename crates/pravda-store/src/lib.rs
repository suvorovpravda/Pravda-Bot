pub mod doctor;
mod migration;
pub mod order_journal;
pub use migration::{MigrationError, MigrationReport};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("invalid market tick")]
    InvalidTick,
    #[error("rusqlite error: {0}")]
    Rusqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct PravdaStore {
    conn: Connection,
}

#[derive(Debug, Clone)]
pub struct HistoryTick {
    pub price: f64,
    pub timestamp: String,
}

impl PravdaStore {
    /// Return the newest bounded window in chronological order.
    pub fn history(
        &self,
        symbol: &str,
        exchange: &str,
        since: DateTime<Utc>,
        until: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<HistoryTick>, StoreError> {
        if !(1..=10_001).contains(&limit) || since > until {
            return Err(StoreError::InvalidTick);
        }
        let mut statement = self.conn.prepare(
            "SELECT price, ts FROM ticks WHERE symbol = ?1 AND exchange = ?2
             AND julianday(ts) >= julianday(?3) AND julianday(ts) <= julianday(?4)
             ORDER BY julianday(ts) DESC, id DESC LIMIT ?5",
        )?;
        let rows = statement.query_map(
            params![
                symbol,
                exchange,
                since.to_rfc3339(),
                until.to_rfc3339(),
                limit as i64
            ],
            |row| {
                Ok(HistoryTick {
                    price: row.get(0)?,
                    timestamp: row.get(1)?,
                })
            },
        )?;
        let mut ticks = rows.collect::<Result<Vec<_>, _>>()?;
        ticks.reverse();
        Ok(ticks)
    }
    pub fn open(path: &str) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=5000;
             CREATE TABLE IF NOT EXISTS ticks (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 symbol TEXT NOT NULL,
                 price REAL NOT NULL,
                 exchange TEXT NOT NULL,
                 ts TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_ticks_symbol_ts ON ticks(symbol, ts);
             CREATE INDEX IF NOT EXISTS idx_ticks_history ON ticks(symbol, exchange, julianday(ts) DESC, id DESC);
         ",
        )?;
        Ok(Self { conn })
    }

    pub fn insert_tick(
        &self,
        symbol: &str,
        price: f64,
        exchange: &str,
        ts: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        if !price.is_finite() || price <= 0.0 {
            return Err(StoreError::InvalidTick);
        }
        self.conn.execute(
            "INSERT INTO ticks (symbol, price, exchange, ts) VALUES (?1, ?2, ?3, ?4)",
            params![symbol, price, exchange, ts.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn latest_ticks(
        &self,
        symbol: &str,
        limit: usize,
    ) -> Result<Vec<(f64, String)>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT price, ts FROM ticks WHERE symbol = ?1 ORDER BY ts DESC LIMIT ?2")?;
        let rows = stmt.query_map(params![symbol, limit as i64], |row| {
            let price: f64 = row.get(0)?;
            let ts: String = row.get(1)?;
            Ok((price, ts))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn history_is_bounded_chronological_and_excludes_future_ticks() {
        let store = PravdaStore::open(":memory:").unwrap();
        let now = Utc::now();
        for seconds in [-3_i64, -2, -1, 1] {
            store
                .insert_tick(
                    "SOL/USDT",
                    (seconds + 100) as f64,
                    "Binance",
                    now + chrono::Duration::seconds(seconds),
                )
                .unwrap();
        }
        let rows = store
            .history(
                "SOL/USDT",
                "Binance",
                now - chrono::Duration::days(1),
                now,
                2,
            )
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].price, 98.0);
        assert_eq!(rows[1].price, 99.0);
        assert!(store
            .history(
                "SOL/USDT",
                "Kraken",
                now - chrono::Duration::days(1),
                now,
                2
            )
            .unwrap()
            .is_empty());
        assert!(store
            .history("SOL/USDT", "Binance", now, now, usize::MAX)
            .is_err());
    }

    #[test]
    fn existing_file_uses_wal_and_survives_reopen_without_schema_migration() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("existing.db");
        let now = Utc::now();
        {
            let store = PravdaStore::open(path.to_str().unwrap()).unwrap();
            store
                .insert_tick("BTC/USDT", 12345.0, "Kraken", now)
                .unwrap();
            let mode: String = store
                .conn
                .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
                .unwrap();
            assert_eq!(mode.to_ascii_lowercase(), "wal");
        }
        // An idle WAL sidecar may be checkpointed and removed on clean close.
        // SQLite's journal mode and row persistence, not file existence, are
        // the durable acceptance criteria.
        let reopened = PravdaStore::open(path.to_str().unwrap()).unwrap();
        let mode: String = reopened
            .conn
            .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(mode.to_ascii_lowercase(), "wal");
        let rows = reopened.latest_ticks("BTC/USDT", 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, 12345.0);
    }

    /// A live WAL database must support a transactionally consistent SQLite
    /// backup without copying db/-wal/-shm files or shutting down the writer.
    /// This proves the snapshot itself restores market rows; it does not
    /// introduce a production backup/restore command.
    #[test]
    fn live_wal_snapshot_restores_rows_without_changing_source() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("live.db");
        let snapshot = dir.path().join("snapshot.db");
        let restored_path = dir.path().join("restored.db");
        let now = Utc::now();
        let store = PravdaStore::open(source.to_str().unwrap()).unwrap();
        for (symbol, price) in [("BTC/USDT", 12345.0), ("ETH/USDT", 2345.0)] {
            store.insert_tick(symbol, price, "Kraken", now).unwrap();
        }
        let mode: String = store
            .conn
            .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(mode, "wal");

        // VACUUM INTO reads a consistent SQLite snapshot while WAL is active.
        // The output must be a NEW file; no overwrite of an existing backup.
        store
            .conn
            .execute("VACUUM main INTO ?1", params![snapshot.to_str().unwrap()])
            .unwrap();
        let snapshot_check: String = Connection::open(&snapshot)
            .unwrap()
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .unwrap();
        assert_eq!(snapshot_check, "ok");
        assert!(store
            .conn
            .execute("VACUUM main INTO ?1", params![snapshot.to_str().unwrap()])
            .is_err());

        // A later tick must not retroactively appear in the saved snapshot.
        store
            .insert_tick("BTC/USDT", 99999.0, "Kraken", now)
            .unwrap();
        std::fs::copy(&snapshot, &restored_path).unwrap();
        let restored = PravdaStore::open(restored_path.to_str().unwrap()).unwrap();
        let btc = restored.latest_ticks("BTC/USDT", 10).unwrap();
        let eth = restored.latest_ticks("ETH/USDT", 10).unwrap();
        assert_eq!(btc.len(), 1);
        assert_eq!(btc[0].0, 12345.0);
        assert_eq!(eth.len(), 1);
        assert_eq!(eth[0].0, 2345.0);
        assert_eq!(store.latest_ticks("BTC/USDT", 10).unwrap().len(), 2);
        assert_eq!(store.latest_ticks("ETH/USDT", 10).unwrap().len(), 1);
    }

    #[test]
    fn test_store_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let store = PravdaStore::open(path.to_str().unwrap()).unwrap();
        let now = Utc::now();
        store
            .insert_tick("BTC/USDT", 12345.0, "Binance", now)
            .unwrap();
        let rows = store.latest_ticks("BTC/USDT", 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, 12345.0);
    }
}
