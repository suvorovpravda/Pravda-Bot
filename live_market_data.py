import json
import math
import os
import threading
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path

from secure_http import SecureHTTPError, get_json
from security_config import SERVICE_HOSTS


DEFAULT_SNAPSHOT_PATH = Path("market_snapshot.json")
FOREX_API_URL = "https://api.frankfurter.dev/v2/rates"
FOREX_QUOTES = ("USD", "GBP", "CHF", "JPY", "CAD", "AUD")
FOREX_CACHE_SECONDS = 21600

_snapshot_lock = threading.Lock()
_forex_lock = threading.Lock()
_last_snapshot_write = 0.0
_forex_cache = None
_forex_cache_updated_at = 0.0


def write_market_snapshot(
    latest_prices,
    last_price_update_at,
    *,
    path=DEFAULT_SNAPSHOT_PATH,
    now_monotonic=None,
    now_utc=None,
    minimum_write_interval=0.5,
):
    global _last_snapshot_write

    monotonic_now = time.monotonic() if now_monotonic is None else now_monotonic
    utc_now = now_utc or datetime.now(timezone.utc)
    snapshot = {
        "received_at": utc_now.isoformat(timespec="milliseconds").replace("+00:00", "Z"),
        "symbols": {},
    }
    for symbol in sorted(set().union(*(exchange.keys() for exchange in latest_prices.values()))):
        quotes = {}
        for exchange_name, prices in latest_prices.items():
            price = prices.get(symbol)
            updated_at = last_price_update_at.get(exchange_name, {}).get(symbol)
            if (
                isinstance(price, (int, float))
                and not isinstance(price, bool)
                and math.isfinite(price)
                and price > 0
                and isinstance(updated_at, (int, float))
                and not isinstance(updated_at, bool)
                and math.isfinite(updated_at)
                and updated_at <= monotonic_now
            ):
                age_seconds = monotonic_now - updated_at
                received_at = utc_now - timedelta(seconds=age_seconds)
                quotes[exchange_name] = {
                    "price": price,
                    "received_at": received_at.isoformat(timespec="milliseconds").replace(
                        "+00:00", "Z"
                    ),
                }
        if quotes:
            snapshot["symbols"][symbol] = quotes

    target = Path(path)
    with _snapshot_lock:
        if monotonic_now - _last_snapshot_write < minimum_write_interval:
            return False
        temporary = target.with_name(f"{target.name}.tmp")
        try:
            target.parent.mkdir(parents=True, exist_ok=True)
            temporary.write_text(
                json.dumps(snapshot, separators=(",", ":"), allow_nan=False),
                encoding="utf-8",
            )
            os.replace(temporary, target)
        finally:
            if temporary.exists():
                temporary.unlink()
        _last_snapshot_write = monotonic_now
    return True


def read_market_snapshot(
    path=DEFAULT_SNAPSHOT_PATH,
    *,
    stale_after_seconds=10.0,
    now=None,
):
    target = Path(path)
    try:
        payload = json.loads(target.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return {
            "status": "waiting",
            "source": "Kraken + Binance public ticker streams",
            "symbols": {},
            "notice": "Lance le bot pour recevoir les cotations en direct.",
        }
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError("market snapshot is unreadable") from error

    if not isinstance(payload, dict) or not isinstance(payload.get("symbols"), dict):
        raise ValueError("market snapshot has an invalid schema")
    current = now or datetime.now(timezone.utc)
    result = {}
    any_fresh = False
    for symbol, raw_quotes in payload["symbols"].items():
        if not isinstance(symbol, str) or not isinstance(raw_quotes, dict):
            continue
        quotes = {}
        for exchange_name, quote in raw_quotes.items():
            if not isinstance(quote, dict):
                continue
            price = quote.get("price")
            timestamp = quote.get("received_at")
            if (isinstance(price, bool) or not isinstance(price, (int, float))
                    or not math.isfinite(price) or price <= 0):
                continue
            if not isinstance(timestamp, str):
                continue
            try:
                received_at = datetime.fromisoformat(timestamp.replace("Z", "+00:00"))
                if received_at.tzinfo is None:
                    received_at = received_at.replace(tzinfo=timezone.utc)
            except ValueError:
                continue
            age_seconds = (current - received_at).total_seconds()
            if age_seconds < 0:
                continue
            fresh = age_seconds <= stale_after_seconds
            any_fresh = any_fresh or fresh
            quotes[exchange_name] = {
                "price": price,
                "received_at": timestamp,
                "age_seconds": round(age_seconds, 2),
                "status": "live" if fresh else "stale",
            }
        if quotes:
            result[symbol] = quotes
    return {
        "status": "live" if any_fresh else "stale" if result else "waiting",
        "source": "Kraken + Binance public ticker streams",
        "retrieved_at": current.isoformat(timespec="seconds"),
        "symbols": result,
        "notice": "Spot last-trade prices from public exchange ticker streams; not executable quotes.",
    }


def get_reference_fx_rates(*, now=None, force_refresh=False):
    global _forex_cache, _forex_cache_updated_at

    current_monotonic = time.monotonic()
    current_utc = now or datetime.now(timezone.utc)
    if current_utc.tzinfo is None:
        raise ValueError("FX reference timestamp must include a timezone")
    current_utc = current_utc.astimezone(timezone.utc)

    def with_freshness(payload):
        source_date = datetime.strptime(payload["date"], "%Y-%m-%d").date()
        source_age_days = max(0, (current_utc.date() - source_date).days)
        result = dict(payload)
        result["freshness"] = "stale" if source_age_days > 4 else "delayed"
        result["source_age_days"] = source_age_days
        result["source_age_seconds"] = source_age_days * 86400
        return result

    with _forex_lock:
        if (
            not force_refresh
            and _forex_cache is not None
            and current_monotonic - _forex_cache_updated_at < FOREX_CACHE_SECONDS
        ):
            return with_freshness(_forex_cache)

    try:
        records = get_json(
            FOREX_API_URL,
            allowed_hosts=SERVICE_HOSTS["fx"],
            params={"base": "EUR", "quotes": ",".join(FOREX_QUOTES)},
            headers={"User-Agent": "Pravda-Bot-Market-Dashboard/1.0"},
            timeout=10,
        )
    except SecureHTTPError:
        raise RuntimeError("reference FX source unavailable") from None
    if not isinstance(records, list):
        raise RuntimeError("reference FX source returned an invalid response")

    rates = {}
    dates = []
    for record in records:
        if not isinstance(record, dict):
            continue
        quote = record.get("quote")
        rate = record.get("rate")
        rate_date = record.get("date")
        if (
            quote in FOREX_QUOTES
            and isinstance(rate, (int, float))
            and not isinstance(rate, bool)
            and math.isfinite(rate)
            and rate > 0
            and isinstance(rate_date, str)
        ):
            rates[quote] = rate
            dates.append(rate_date)
    if len(rates) != len(FOREX_QUOTES) or not dates:
        raise RuntimeError("reference FX source omitted expected currency rates")

    as_of = max(dates)
    try:
        source_date = datetime.strptime(as_of, "%Y-%m-%d").date()
    except ValueError as error:
        raise RuntimeError("reference FX source returned an invalid rate date") from error
    retrieved_at = current_utc
    source_age_days = max(0, (retrieved_at.date() - source_date).days)
    result = {
        "status": "available",
        "freshness": "stale" if source_age_days > 4 else "delayed",
        "base": "EUR",
        "date": as_of,
        "retrieved_at": retrieved_at.isoformat(timespec="seconds").replace("+00:00", "Z"),
        "source_age_days": source_age_days,
        "source_age_seconds": source_age_days * 86400,
        "rates": rates,
        "source": "Frankfurter / European Central Bank reference rates",
        "source_url": "https://www.frankfurter.app/",
        "notice": (
            "Daily reference rates, not live interbank FX quotes. "
            f"Source date is {source_age_days} calendar days old."
        ),
    }
    with _forex_lock:
        _forex_cache = result
        _forex_cache_updated_at = time.monotonic()
    return with_freshness(result)
