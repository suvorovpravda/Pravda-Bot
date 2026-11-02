//! Atomic import of archived Python market history into the Rust store.
//! The legacy database is read only. Only the destination is committed.
use std::path::Path;

use chrono::DateTime;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, TransactionBehavior};
use thiserror::Error;

use crate::PravdaStore;

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("cannot access the legacy database: {0}")]
    LegacyIo(#[from] std::io::Error),
    #[error("cannot migrate a database into itself")]
    SameDatabase,
    #[error("SQLite migration failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("legacy row {0} contains invalid market data")]
    InvalidRow(i64),
    #[error("previously imported legacy row {0} was changed or deleted in the target")]
    ChangedRow(i64),
    #[error("legacy market_ticks table has an incompatible schema")]
    InvalidLegacySchema,
    #[error("the Rust ticks table is missing or incompatible")]
    InvalidTargetSchema,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MigrationReport {
    pub source_rows: u64,
    pub inserted: u64,
    pub already_imported: u64,
}

/// Returns true only if a table contains every required column.
fn has_columns(conn: &Connection, table: &str, required: &[&str]) -> Result<bool, rusqlite::Error> {
    // Table names are code-owned constants, never interpolated from input.
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(required
        .iter()
        .all(|column| names.iter().any(|name| name == column)))
}

impl PravdaStore {
    /// Import archived Python rows exactly once per canonical path and original ID.
    ///
    /// The source is opened read-only, never created, modified, or deleted.
    /// An IMMEDIATE transaction covers all target inserts and provenance records.
    /// Replays validate previously imported values. Keep source path stable.
    pub fn migrate_legacy_schema(
        &mut self,
        legacy_path: &Path,
    ) -> Result<MigrationReport, MigrationError> {
        let source_path = legacy_path.canonicalize()?;
        let target_file: String = self.conn.query_row(
            "SELECT file FROM pragma_database_list WHERE name = 'main'",
            [],
            |row| row.get(0),
        )?;
        if !target_file.is_empty()
            && std::path::Path::new(&target_file).canonicalize()? == source_path
        {
            return Err(MigrationError::SameDatabase);
        }

        // Never create an empty source by accident.
        let mut source =
            Connection::open_with_flags(&source_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let exists: bool = source.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'market_ticks')",
            [],
            |row| row.get(0),
        )?;
        if !exists {
            return Ok(MigrationReport::default());
        }
        if !has_columns(
            &source,
            "market_ticks",
            &["id", "timestamp_utc", "symbol", "exchange", "price"],
        )? {
            return Err(MigrationError::InvalidLegacySchema);
        }
        if !has_columns(
            &self.conn,
            "ticks",
            &["id", "ts", "symbol", "exchange", "price"],
        )? {
            return Err(MigrationError::InvalidTargetSchema);
        }

        let source_identity = source_path.to_string_lossy().into_owned();
        // This read transaction pins a stable snapshot of the legacy database.
        let source_tx = source.transaction_with_behavior(TransactionBehavior::Deferred)?;
        let mut stmt = source_tx.prepare(
            "SELECT id, timestamp_utc, symbol, exchange, price FROM market_ticks ORDER BY id",
        )?;
        let mut rows = stmt.query([])?;

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS legacy_tick_imports (
                source_path TEXT NOT NULL,
                legacy_tick_id INTEGER NOT NULL,
                tick_id INTEGER NOT NULL,
                PRIMARY KEY(source_path, legacy_tick_id),
                FOREIGN KEY(tick_id) REFERENCES ticks(id)
            );",
        )?;

        let mut report = MigrationReport::default();
        while let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let timestamp: String = row.get(1)?;
            let symbol: String = row.get(2)?;
            let exchange: String = row.get(3)?;
            let price: f64 = row.get(4)?;

            if !price.is_finite()
                || price <= 0.0
                || symbol.trim().is_empty()
                || exchange.trim().is_empty()
                || DateTime::parse_from_rfc3339(&timestamp).is_err()
            {
                return Err(MigrationError::InvalidRow(id));
            }

            report.source_rows += 1;
            let prior: Option<(String, f64, String, String)> = tx
                .query_row(
                    "SELECT t.symbol, t.price, t.exchange, t.ts
                     FROM legacy_tick_imports AS m
                     JOIN ticks AS t ON t.id = m.tick_id
                     WHERE m.source_path = ?1 AND m.legacy_tick_id = ?2",
                    params![source_identity, id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .optional()?;

            if let Some((old_symbol, old_price, old_exchange, old_ts)) = prior {
                if old_symbol != symbol
                    || old_price != price
                    || old_exchange != exchange
                    || old_ts != timestamp
                {
                    return Err(MigrationError::ChangedRow(id));
                }
                report.already_imported += 1;
                continue;
            }

            // A dangling provenance record is a fatal integrity error.
            let mapped: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM legacy_tick_imports
                 WHERE source_path = ?1 AND legacy_tick_id = ?2)",
                params![source_identity, id],
                |r| r.get(0),
            )?;
            if mapped {
                return Err(MigrationError::ChangedRow(id));
            }

            tx.execute(
                "INSERT INTO ticks(symbol, price, exchange, ts) VALUES (?1, ?2, ?3, ?4)",
                params![symbol, price, exchange, timestamp],
            )?;
            let tick_id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO legacy_tick_imports(source_path, legacy_tick_id, tick_id)
                 VALUES (?1, ?2, ?3)",
                params![source_identity, id, tick_id],
            )?;
            report.inserted += 1;
        }

        // The only commit, after all source rows and mapped values are checked.
        tx.commit()?;
        Ok(report)
    }
}
