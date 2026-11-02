# Task 3 — Transactional legacy Python SQLite history import

**Scope:** Python market_ticks -> Rust ticks. No frontend, Tauri configuration,
or destructive database changes.

## Database contracts

| Python legacy | Rust target |
| --- | --- |
| MARKET_HISTORY_DB, usually market_history.sqlite3 | PRAVDA_DB_PATH; Windows desktop default under LOCALAPPDATA/Pravda-Bot/pravda.db |
| market_ticks(id, timestamp_utc, symbol, exchange, price) | ticks(id, symbol, price, exchange, ts) |
| Original id identifies the event | Auto-generated id; provenance kept in legacy_tick_imports(source_path, legacy_tick_id, tick_id) |

The source is opened **read-only**, never modified, dropped, moved or deleted.
Original timestamps are copied verbatim after RFC3339 validation; price, symbol
and exchange are preserved. Different legacy IDs remain different Rust ticks
even if their price, symbol and time are identical. Pre-existing Rust rows stay
intact.

The Python price_alerts and source_observations tables are **not** migrated
by Task 3; they require separate reviewed contracts.

## Source selection

The server looks for the source in this order:

1. PRAVDA_LEGACY_DB_PATH: explicitly configured path (recommended on Windows).
2. MARKET_HISTORY_DB: legacy Python path if explicitly provided.
3. market_history.sqlite3 in the server working directory, only if it exists.

The desktop working directory is not a dependable location for older Python
databases. Prefer an explicit full path. Example PowerShell:

    $env:PRAVDA_LEGACY_DB_PATH = "C:\path\to\old\market_history.sqlite3"

An absent, non-configured default is a no-op. An explicitly selected but
missing or unreadable source is a fatal startup error, not an empty database
creation. Source without market_ticks is a no-op; incompatible schema fails.
The source and target must be different files.

The canonical source file path is part of the replay identity. **Keep the same
path across launches:** relocating the source makes the importer treat its
events as a different source, potentially duplicating them.

## Atomic execution and startup

PravdaStore::open sets up the target ticks table and WAL journal. In
pravda_server::run_server, after opening the store but *before* constructing
PravdaRuntime or starting market streams, the server performs one migration
attempt.

The importer opens a read transaction on the legacy file and a single
IMMEDIATE write transaction on the Rust target. Within the target transaction,
it creates the provenance table, validates each legacy row and inserts new ticks
with their original IDs in provenance. Previously imported rows are verified
against their destination values; altered or dangling mappings are errors.

Only after *all* rows pass is COMMIT called. Errors trigger rollback of all
uncommitted target inserts and provenance; no source rows are altered. An error
stops application startup before the market workers start. The legacy source
is preserved for re-examination and recovery.

WAL provides transactional atomicity. Crash/power-loss durability also depends
on SQLite synchronous mode and storage guarantees; SQL transactions do not
eliminate every risk of physical disk failure. For production, take and verify
a **consistent backup of both databases** before migration. Do not copy a
live WAL database file without also capturing its WAL correctly; use SQLite's
backup API or an offline backup.

## Idempotence and rollback

A source row is identified by canonical source path and original integer id.
Repeated startup of the same unchanged source imports zero duplicates.
Newly appended legacy rows can be imported incrementally.

There is no automatic deletion of imported rows. To revert a committed import
without affecting other Rust data, restore a verified pre-import snapshot of
the target database. Preserve the legacy database at all times.

## Acceptance tests

Integration tests use rusqlite and tempfile. They cover:
- Duplicate-valued independent events, preservation of existing Rust ticks
- Repeated import, reopen, and incremental source append
- Missing source, no market_ticks, invalid schema, and same-file rejection
- Invalid legacy timestamps and all-or-nothing rollback
- Injected SQLite error during import, recovery on rerun
- Mutation of an already imported legacy event

Human local build verification, CI Rust Linux/Windows/MSRV/Desktop and CodeQL
must pass before merge. No frontend/HTML/CSS/JavaScript/Tauri modifications.
Branch-protection configuration remains a separate governance issue in #55.
