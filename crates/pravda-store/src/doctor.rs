//! Read-only SQLite production health probe.
//!
//! Never creates or repairs a database, reveals paths, dumps rows, reads
//! secrets, or changes market/trading state.
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseHealth {
    Healthy,
    Missing,
    NotAFile,
    Unavailable,
    Corrupt,
    SchemaMissing,
}

impl DatabaseHealth {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Healthy => "database_ok",
            Self::Missing => "database_missing",
            Self::NotAFile => "database_invalid_path",
            Self::Unavailable => "database_unavailable",
            Self::Corrupt => "database_integrity_failed",
            Self::SchemaMissing => "database_schema_missing",
        }
    }

    pub const fn is_healthy(self) -> bool {
        matches!(self, Self::Healthy)
    }
}

/// Probe SQLite with SQLITE_OPEN_READ_ONLY and no schema changes.
/// A read-only connection still takes SQLite locks as necessary; it is not an
/// exclusive maintenance transaction. Only safe, bounded PRAGMA is executed.
pub fn inspect_database(path: &Path) -> DatabaseHealth {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(value) => value,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return DatabaseHealth::Missing;
        }
        Err(_) => return DatabaseHealth::Unavailable,
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return DatabaseHealth::NotAFile;
    }
    let conn = match Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        Ok(connection) => connection,
        Err(_) => return DatabaseHealth::Unavailable,
    };
    if conn.busy_timeout(std::time::Duration::from_millis(750)).is_err() {
        return DatabaseHealth::Unavailable;
    }
    let integrity: String =
        match conn.query_row("PRAGMA quick_check(1)", [], |row| row.get(0)) {
            Ok(result) => result,
            Err(_) => return DatabaseHealth::Corrupt,
        };
    if integrity != "ok" {
        return DatabaseHealth::Corrupt;
    }
    let table_count: i64 = match conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='ticks'",
        [],
        |row| row.get(0),
    ) {
        Ok(count) => count,
        Err(_) => return DatabaseHealth::Unavailable,
    };
    if table_count != 1 {
        return DatabaseHealth::SchemaMissing;
    }
    DatabaseHealth::Healthy
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_file_is_never_created() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing.db");
        assert_eq!(inspect_database(&path), DatabaseHealth::Missing);
        assert!(!path.exists());
    }

    #[test]
    fn health_check_is_read_only_and_works_on_existing_store() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let store = crate::PravdaStore::open(path.to_str().unwrap()).unwrap();
        store
            .insert_tick("BTC/USDT", 100.0, "Kraken", chrono::Utc::now())
            .unwrap();
        assert_eq!(inspect_database(&path), DatabaseHealth::Healthy);
        assert_eq!(store.latest_ticks("BTC/USDT", 10).unwrap().len(), 1);
    }

    #[test]
    fn invalid_data_and_schema_are_reported_without_repairs() {
        let dir = tempdir().unwrap();
        let invalid = dir.path().join("bad.db");
        std::fs::write(&invalid, b"not a SQLite database").unwrap();
        assert_eq!(inspect_database(&invalid), DatabaseHealth::Corrupt);

        let empty = dir.path().join("empty.db");
        {
            let _connection = Connection::open(&empty).unwrap();
        }
        assert_eq!(inspect_database(&empty), DatabaseHealth::SchemaMissing);
    }

    #[test]
    fn directories_are_not_opened_as_databases() {
        let dir = tempdir().unwrap();
        assert_eq!(inspect_database(dir.path()), DatabaseHealth::NotAFile);
    }
}
