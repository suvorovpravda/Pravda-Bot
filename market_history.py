import csv
from contextlib import contextmanager
import hashlib
import json
import io
import math
import os
import sqlite3
import threading
from datetime import datetime, timedelta, timezone
from pathlib import Path


DEFAULT_DB_PATH = Path("market_history.sqlite3")
SUPPORTED_SYMBOLS = {"BTC/USDT", "ETH/USDT", "SOL/USDT"}
SUPPORTED_EXCHANGES = {"Binance", "Kraken"}
PERIODS = {"day": timedelta(days=1), "week": timedelta(days=7), "month": timedelta(days=30)}
_schema_lock = threading.Lock()


class MarketStore:
    def __init__(self, path=None):
        self.path = Path(path or os.environ.get("MARKET_HISTORY_DB", DEFAULT_DB_PATH))
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with _schema_lock, self._connection() as connection:
            connection.execute("PRAGMA journal_mode=WAL")
            connection.executescript(
                """
                CREATE TABLE IF NOT EXISTS market_ticks (
                    id INTEGER PRIMARY KEY,
                    timestamp_utc TEXT NOT NULL,
                    symbol TEXT NOT NULL,
                    exchange TEXT NOT NULL,
                    price REAL NOT NULL CHECK(price > 0)
                );
                CREATE INDEX IF NOT EXISTS idx_market_ticks_lookup
                    ON market_ticks(symbol, exchange, timestamp_utc);
                CREATE TABLE IF NOT EXISTS price_alerts (
                    id INTEGER PRIMARY KEY,
                    symbol TEXT NOT NULL,
                    exchange TEXT NOT NULL DEFAULT 'Binance',
                    currency TEXT NOT NULL CHECK(currency IN ('EUR', 'CHF')),
                    direction TEXT NOT NULL CHECK(direction IN ('above', 'below')),
                    target REAL NOT NULL CHECK(target > 0),
                    enabled INTEGER NOT NULL DEFAULT 1,
                    condition_met INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS source_observations (
                    id INTEGER PRIMARY KEY,
                    kind TEXT NOT NULL,
                    source TEXT NOT NULL,
                    event_at TEXT,
                    first_seen_at TEXT NOT NULL,
                    fingerprint TEXT NOT NULL UNIQUE,
                    payload_json TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_source_observations_kind_date
                    ON source_observations(kind, event_at, first_seen_at);
                """
            )
            columns = {
                row["name"]
                for row in connection.execute("PRAGMA table_info(price_alerts)")
            }
            if "exchange" not in columns:
                connection.execute(
                    "ALTER TABLE price_alerts ADD COLUMN exchange TEXT NOT NULL DEFAULT 'Binance'"
                )

    def _connect(self):
        connection = sqlite3.connect(self.path, timeout=5)
        connection.row_factory = sqlite3.Row
        connection.execute("PRAGMA busy_timeout=5000")
        return connection

    @contextmanager
    def _connection(self):
        connection = self._connect()
        try:
            yield connection
            connection.commit()
        except Exception:
            connection.rollback()
            raise
        finally:
            connection.close()

    @staticmethod
    def _timestamp(value):
        if value is None:
            value = datetime.now(timezone.utc)
        if value.tzinfo is None:
            raise ValueError("market tick timestamp must include a timezone")
        return value.astimezone(timezone.utc).isoformat(timespec="milliseconds").replace(
            "+00:00", "Z"
        )

    def record_tick(self, symbol, exchange, price, *, timestamp=None):
        if symbol not in SUPPORTED_SYMBOLS or exchange not in SUPPORTED_EXCHANGES:
            raise ValueError("unsupported market symbol or exchange")
        if not isinstance(price, (int, float)) or not math.isfinite(price) or price <= 0:
            raise ValueError("market tick price must be finite and positive")
        with self._connection() as connection:
            connection.execute(
                "INSERT INTO market_ticks(timestamp_utc, symbol, exchange, price) VALUES (?, ?, ?, ?)",
                (self._timestamp(timestamp), symbol, exchange, float(price)),
            )

    def history(self, symbol, exchange, period, *, now=None):
        if symbol not in SUPPORTED_SYMBOLS or exchange not in SUPPORTED_EXCHANGES:
            raise ValueError("unsupported market symbol or exchange")
        if period not in PERIODS:
            raise ValueError("period must be day, week, or month")
        current = now or datetime.now(timezone.utc)
        if current.tzinfo is None:
            raise ValueError("current time must include a timezone")
        start = current.astimezone(timezone.utc) - PERIODS[period]
        start_text = self._timestamp(start)
        end_text = self._timestamp(current)
        bucket_seconds = {"day": 60, "week": 900, "month": 3600}[period]
        with self._connection() as connection:
            rows = connection.execute(
                """
                SELECT CAST(strftime('%s', timestamp_utc) AS INTEGER) / ? * ? AS bucket,
                       AVG(price) AS price, MIN(price) AS low, MAX(price) AS high,
                       COUNT(*) AS ticks
                FROM market_ticks
                WHERE symbol = ? AND exchange = ? AND timestamp_utc >= ? AND timestamp_utc <= ?
                GROUP BY bucket ORDER BY bucket
                """,
                (bucket_seconds, bucket_seconds, symbol, exchange, start_text, end_text),
            ).fetchall()
        return [
            {
                "timestamp": datetime.fromtimestamp(row["bucket"], timezone.utc)
                .isoformat(timespec="seconds")
                .replace("+00:00", "Z"),
                "price": row["price"],
                "low": row["low"],
                "high": row["high"],
                "ticks": row["ticks"],
            }
            for row in rows
        ]

    def export_history(self, symbol, exchange, period, *, now=None):
        if symbol not in SUPPORTED_SYMBOLS or exchange not in SUPPORTED_EXCHANGES:
            raise ValueError("unsupported market symbol or exchange")
        if period not in PERIODS:
            raise ValueError("period must be day, week, or month")
        current = now or datetime.now(timezone.utc)
        if current.tzinfo is None:
            raise ValueError("current time must include a timezone")
        start = self._timestamp(current.astimezone(timezone.utc) - PERIODS[period])
        end = self._timestamp(current)
        with self._connection() as connection:
            rows = connection.execute(
                """
                SELECT timestamp_utc, price FROM market_ticks
                WHERE symbol = ? AND exchange = ? AND timestamp_utc >= ? AND timestamp_utc <= ?
                ORDER BY timestamp_utc, id
                """,
                (symbol, exchange, start, end),
            ).fetchall()
        output = io.StringIO(newline="")
        writer = csv.writer(output)
        writer.writerow(["Timestamp_UTC", "Symbol", "Exchange", "Price_USDT"])
        writer.writerows(
            [row["timestamp_utc"], symbol, exchange, row["price"]]
            for row in rows
        )
        return output.getvalue()

    def daily_market_summary(self, day):
        start = datetime.combine(day, datetime.min.time(), tzinfo=timezone.utc)
        end = start + timedelta(days=1)
        with self._connection() as connection:
            rows = connection.execute(
                """
                SELECT symbol, exchange, COUNT(*) AS ticks, MIN(price) AS low,
                       MAX(price) AS high,
                       (SELECT price FROM market_ticks first
                        WHERE first.symbol = market_ticks.symbol AND first.exchange = market_ticks.exchange
                          AND first.timestamp_utc >= ? AND first.timestamp_utc < ?
                        ORDER BY timestamp_utc, id LIMIT 1) AS open,
                       (SELECT price FROM market_ticks last
                        WHERE last.symbol = market_ticks.symbol AND last.exchange = market_ticks.exchange
                          AND last.timestamp_utc >= ? AND last.timestamp_utc < ?
                        ORDER BY timestamp_utc DESC, id DESC LIMIT 1) AS close
                FROM market_ticks
                WHERE timestamp_utc >= ? AND timestamp_utc < ?
                GROUP BY symbol, exchange
                """,
                (
                    self._timestamp(start),
                    self._timestamp(end),
                    self._timestamp(start),
                    self._timestamp(end),
                    self._timestamp(start),
                    self._timestamp(end),
                ),
            ).fetchall()
        return [dict(row) for row in rows]

    def list_alerts(self):
        with self._connection() as connection:
            rows = connection.execute(
                "SELECT id, symbol, exchange, currency, direction, target, enabled FROM price_alerts ORDER BY id"
            ).fetchall()
        return [dict(row) for row in rows]

    def create_alert(self, symbol, exchange, currency, direction, target):
        if symbol not in SUPPORTED_SYMBOLS:
            raise ValueError("unsupported market symbol")
        if exchange not in SUPPORTED_EXCHANGES:
            raise ValueError("unsupported market exchange")
        if currency not in {"EUR", "CHF"} or direction not in {"above", "below"}:
            raise ValueError("unsupported alert currency or direction")
        if not isinstance(target, (int, float)) or not math.isfinite(target) or target <= 0:
            raise ValueError("alert target must be finite and positive")
        with self._connection() as connection:
            count = connection.execute("SELECT COUNT(*) FROM price_alerts").fetchone()[0]
            if count >= 50:
                raise ValueError("at most 50 alerts may be configured")
            cursor = connection.execute(
                """
                INSERT INTO price_alerts(symbol, exchange, currency, direction, target, created_at)
                VALUES (?, ?, ?, ?, ?, ?)
                """,
                (
                    symbol,
                    exchange,
                    currency,
                    direction,
                    float(target),
                    self._timestamp(None),
                ),
            )
            alert_id = cursor.lastrowid
        return next(alert for alert in self.list_alerts() if alert["id"] == alert_id)

    def delete_alert(self, alert_id):
        with self._connection() as connection:
            cursor = connection.execute("DELETE FROM price_alerts WHERE id = ?", (alert_id,))
        return cursor.rowcount > 0

    def record_source_events(self, kind, source, rows, *, retrieved_at=None):
        if kind not in {"energy_flow_observation", "reported_news"}:
            raise ValueError("unsupported source observation type")
        if not isinstance(source, str) or not source:
            raise ValueError("source name must not be empty")
        fetched = self._timestamp(retrieved_at)
        stored = 0
        with self._connection() as connection:
            for row in rows:
                if not isinstance(row, dict):
                    continue
                serialized = json.dumps(
                    {"source": source, **row},
                    ensure_ascii=False,
                    sort_keys=True,
                    separators=(",", ":"),
                    allow_nan=False,
                )
                fingerprint = hashlib.sha256(
                    f"{kind}\0{source}\0{serialized}".encode("utf-8")
                ).hexdigest()
                event_at = (
                    row.get("updated_at")
                    or row.get("period_from")
                    or row.get("published_at")
                    or row.get("event_at")
                )
                event_at = self._normalize_event_timestamp(event_at)
                cursor = connection.execute(
                    """
                    INSERT OR IGNORE INTO source_observations
                        (kind, source, event_at, first_seen_at, fingerprint, payload_json)
                    VALUES (?, ?, ?, ?, ?, ?)
                    """,
                    (kind, source, event_at, fetched, fingerprint, serialized),
                )
                stored += cursor.rowcount
        return stored

    @staticmethod
    def _normalize_event_timestamp(value):
        if not isinstance(value, str) or not value:
            return None
        try:
            parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
        except ValueError:
            try:
                parsed = datetime.strptime(value, "%Y%m%dT%H%M%SZ").replace(
                    tzinfo=timezone.utc
                )
            except ValueError:
                try:
                    parsed = datetime.strptime(value, "%Y-%m-%d").replace(
                        tzinfo=timezone.utc
                    )
                except ValueError:
                    return None
        if parsed.tzinfo is None:
            parsed = parsed.replace(tzinfo=timezone.utc)
        return parsed.astimezone(timezone.utc).isoformat(timespec="seconds").replace(
            "+00:00", "Z"
        )

    def source_events(self, kind, report_date):
        if kind not in {"energy_flow_observation", "reported_news"}:
            raise ValueError("unsupported source observation type")
        start = datetime.combine(report_date, datetime.min.time(), tzinfo=timezone.utc)
        end = start + timedelta(days=1)
        with self._connection() as connection:
            rows = connection.execute(
                """
                SELECT source, event_at, first_seen_at, payload_json
                FROM source_observations
                WHERE kind = ? AND (
                    (event_at >= ? AND event_at < ?)
                    OR (event_at IS NULL AND first_seen_at >= ? AND first_seen_at < ?)
                )
                ORDER BY COALESCE(event_at, first_seen_at)
                """,
                (
                    kind,
                    self._timestamp(start),
                    self._timestamp(end),
                    self._timestamp(start),
                    self._timestamp(end),
                ),
            ).fetchall()
        return [
            {
                "source": row["source"],
                "event_at": row["event_at"],
                "first_seen_at": row["first_seen_at"],
                "payload": json.loads(row["payload_json"]),
            }
            for row in rows
        ]

    def check_alerts(self, snapshot, fx_rates):
        triggered = []
        symbols = snapshot.get("symbols", {})
        rates = fx_rates.get("rates", {})
        usd_per_eur = rates.get("USD")
        chf_per_eur = rates.get("CHF")
        with self._connection() as connection:
            alerts = connection.execute(
                "SELECT * FROM price_alerts WHERE enabled = 1"
            ).fetchall()
            for alert in alerts:
                quote = symbols.get(alert["symbol"], {}).get(alert["exchange"])
                if (
                    not isinstance(quote, dict)
                    or quote.get("status") != "live"
                    or not isinstance(quote.get("price"), (int, float))
                    or not math.isfinite(quote["price"])
                    or quote["price"] <= 0
                    or not isinstance(usd_per_eur, (int, float))
                    or usd_per_eur <= 0
                ):
                    continue
                eur_price = quote["price"] / usd_per_eur
                value = (
                    eur_price
                    if alert["currency"] == "EUR"
                    else eur_price * chf_per_eur
                    if isinstance(chf_per_eur, (int, float)) and chf_per_eur > 0
                    else None
                )
                if value is None:
                    continue
                condition = (
                    value >= alert["target"]
                    if alert["direction"] == "above"
                    else value <= alert["target"]
                )
                if condition and not alert["condition_met"]:
                    triggered.append(
                        {
                            "id": alert["id"],
                            "symbol": alert["symbol"],
                            "exchange": alert["exchange"],
                            "currency": alert["currency"],
                            "direction": alert["direction"],
                            "target": alert["target"],
                            "price": value,
                        }
                    )
                connection.execute(
                    "UPDATE price_alerts SET condition_met = ? WHERE id = ?",
                    (int(condition), alert["id"]),
                )
        return triggered


_market_store = None
_market_store_lock = threading.Lock()


def get_market_store():
    global _market_store
    path = Path(os.environ.get("MARKET_HISTORY_DB", DEFAULT_DB_PATH))
    with _market_store_lock:
        if _market_store is None or _market_store.path != path:
            _market_store = MarketStore(path)
        return _market_store
