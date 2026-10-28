use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("rusqlite error: {0}")]
    Rusqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct PravdaStore {
    conn: Connection,
}

impl PravdaStore {
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
