use std::path::Path;

use pravda_store::{MigrationError, MigrationReport, PravdaStore};
use rusqlite::{params, Connection};
use tempfile::tempdir;

const CREATE_LEGACY: &str = "
    PRAGMA journal_mode=WAL;
    CREATE TABLE market_ticks (
        id INTEGER PRIMARY KEY,
        timestamp_utc TEXT NOT NULL,
        symbol TEXT NOT NULL,
        exchange TEXT NOT NULL,
        price REAL NOT NULL
    );
";

fn legacy_db(path: &Path, ticks: &[(i64, &str, &str, &str, f64)]) {
    let db = Connection::open(path).unwrap();
    db.execute_batch(CREATE_LEGACY).unwrap();
    for &(id, timestamp, symbol, exchange, price) in ticks {
        db.execute(
            "INSERT INTO market_ticks(id, timestamp_utc, symbol, exchange, price)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, timestamp, symbol, exchange, price],
        )
        .unwrap();
    }
}

fn count(db: &Connection, table: &str) -> i64 {
    db.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}

#[test]
fn imports_preserves_values_and_existing_ticks_and_is_idempotent_across_reopen() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("market_history.sqlite3");
    let target = dir.path().join("pravda.db");
    // Two distinct legacy events are allowed to have identical values.
    legacy_db(
        &source,
        &[
            (11, "2026-10-27T12:00:00.000Z", "BTC/USDT", "Kraken", 100.0),
            (12, "2026-10-27T12:00:00.000Z", "BTC/USDT", "Kraken", 100.0),
            (13, "2026-10-27T12:00:01.000Z", "ETH/USDT", "Binance", 230.5),
        ],
    );

    {
        let mut store = PravdaStore::open(target.to_str().unwrap()).unwrap();
        store
            .insert_tick(
                "SOL/USDT",
                55.0,
                "Kraken",
                chrono::DateTime::parse_from_rfc3339("2026-10-27T11:59:00Z")
                    .unwrap()
                    .to_utc(),
            )
            .unwrap();
        assert_eq!(
            store.migrate_legacy_schema(&source).unwrap(),
            MigrationReport {
                source_rows: 3,
                inserted: 3,
                already_imported: 0,
            }
        );
        assert_eq!(
            store.migrate_legacy_schema(&source).unwrap(),
            MigrationReport {
                source_rows: 3,
                inserted: 0,
                already_imported: 3,
            }
        );
    }
    {
        let mut store = PravdaStore::open(target.to_str().unwrap()).unwrap();
        assert_eq!(store.migrate_legacy_schema(&source).unwrap().inserted, 0);
    }

    let db = Connection::open(&target).unwrap();
    assert_eq!(count(&db, "ticks"), 4);
    assert_eq!(count(&db, "legacy_tick_imports"), 3);
    let values: Vec<(String, f64, String, String)> = db
        .prepare(
            "SELECT t.symbol, t.price, t.exchange, t.ts
             FROM ticks AS t JOIN legacy_tick_imports AS m ON m.tick_id = t.id
             ORDER BY m.legacy_tick_id",
        )
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        values,
        vec![
            ("BTC/USDT".into(), 100.0, "Kraken".into(), "2026-10-27T12:00:00.000Z".into()),
            ("BTC/USDT".into(), 100.0, "Kraken".into(), "2026-10-27T12:00:00.000Z".into()),
            ("ETH/USDT".into(), 230.5, "Binance".into(), "2026-10-27T12:00:01.000Z".into()),
        ]
    );
    assert_eq!(count(&Connection::open(&source).unwrap(), "market_ticks"), 3);
}

#[test]
fn supports_incremental_import_without_repeating_previous_rows() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("legacy.db");
    legacy_db(
        &source,
        &[(1, "2026-10-27T12:00:00Z", "BTC/USDT", "Kraken", 100.0)],
    );
    let mut store = PravdaStore::open(dir.path().join("new.db").to_str().unwrap()).unwrap();
    assert_eq!(store.migrate_legacy_schema(&source).unwrap().inserted, 1);
    Connection::open(&source)
        .unwrap()
        .execute(
            "INSERT INTO market_ticks VALUES (2, '2026-10-27T12:01:00Z', 'ETH/USDT', 'Kraken', 200.0)",
            [],
        )
        .unwrap();
    let report = store.migrate_legacy_schema(&source).unwrap();
    assert_eq!(report.inserted, 1);
    assert_eq!(report.already_imported, 1);
}

#[test]
fn no_legacy_table_is_a_safe_noop_and_missing_source_is_an_error() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("other.db");
    Connection::open(&source).unwrap().execute_batch("CREATE TABLE something(id INTEGER);").unwrap();
    let mut store = PravdaStore::open(dir.path().join("new.db").to_str().unwrap()).unwrap();
    assert_eq!(store.migrate_legacy_schema(&source).unwrap(), MigrationReport::default());
    assert!(matches!(
        store.migrate_legacy_schema(&dir.path().join("absent.db")),
        Err(MigrationError::LegacyIo(_))
    ));
    assert!(!dir.path().join("absent.db").exists());
}

#[test]
fn refuses_self_import_and_reports_incompatible_legacy_schema() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("pravda.db");
    let mut store = PravdaStore::open(target.to_str().unwrap()).unwrap();
    assert!(matches!(store.migrate_legacy_schema(&target), Err(MigrationError::SameDatabase)));

    let source = dir.path().join("wrong.db");
    Connection::open(&source).unwrap().execute_batch("CREATE TABLE market_ticks(id INTEGER PRIMARY KEY);").unwrap();
    assert!(matches!(
        store.migrate_legacy_schema(&source),
        Err(MigrationError::InvalidLegacySchema)
    ));
}

#[test]
fn invalid_source_row_rolls_back_entire_import() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("legacy.db");
    let target = dir.path().join("new.db");
    legacy_db(
        &source,
        &[
            (1, "2026-10-27T12:00:00Z", "BTC/USDT", "Kraken", 120.0),
            (2, "invalid time", "ETH/USDT", "Kraken", 150.0),
        ],
    );
    let mut store = PravdaStore::open(target.to_str().unwrap()).unwrap();
    assert!(matches!(store.migrate_legacy_schema(&source), Err(MigrationError::InvalidRow(2))));
    let db = Connection::open(&target).unwrap();
    assert_eq!(count(&db, "ticks"), 0);
    assert_eq!(count(&Connection::open(&source).unwrap(), "market_ticks"), 2);
}

#[test]
fn injected_sqlite_failure_rolls_back_all_prior_inserts() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("legacy.db");
    let target = dir.path().join("new.db");
    legacy_db(
        &source,
        &[
            (1, "2026-10-27T12:00:00Z", "BTC/USDT", "Kraken", 120.0),
            (2, "2026-10-27T12:01:00Z", "SOL/USDT", "Kraken", 150.0),
        ],
    );
    let mut store = PravdaStore::open(target.to_str().unwrap()).unwrap();
    let db = Connection::open(&target).unwrap();
    db.execute_batch(
        "CREATE TRIGGER fail_import BEFORE INSERT ON ticks
         WHEN NEW.symbol = 'SOL/USDT'
         BEGIN SELECT RAISE(ABORT, 'injected migration failure'); END;",
    ).unwrap();
    assert!(matches!(store.migrate_legacy_schema(&source), Err(MigrationError::Sqlite(_))));
    assert_eq!(count(&db, "ticks"), 0);
    db.execute_batch("DROP TRIGGER fail_import;").unwrap();
    assert_eq!(store.migrate_legacy_schema(&source).unwrap().inserted, 2);
    assert_eq!(count(&db, "ticks"), 2);
    assert_eq!(count(&Connection::open(&source).unwrap(), "market_ticks"), 2);
}

#[test]
fn source_mutation_fails_closed_instead_of_silently_discarding_changes() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("legacy.db");
    let target = dir.path().join("new.db");
    legacy_db(
        &source,
        &[(1, "2026-10-27T12:00:00Z", "BTC/USDT", "Kraken", 100.0)],
    );
    let mut store = PravdaStore::open(target.to_str().unwrap()).unwrap();
    store.migrate_legacy_schema(&source).unwrap();
    let legacy = Connection::open(&source).unwrap();
    legacy.execute("UPDATE market_ticks SET price = 999.0 WHERE id = 1", []).unwrap();
    assert!(matches!(store.migrate_legacy_schema(&source), Err(MigrationError::ChangedRow(1))));
    let db = Connection::open(&target).unwrap();
    let price: f64 = db.query_row("SELECT price FROM ticks", [], |r| r.get(0)).unwrap();
    assert_eq!(price, 100.0);
}
