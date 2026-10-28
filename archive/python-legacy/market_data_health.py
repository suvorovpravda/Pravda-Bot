from dataclasses import dataclass
from datetime import datetime, timezone
import math
import time
from typing import Literal


@dataclass(frozen=True)
class MarketDataHealth:
    status: Literal["WAITING", "STALE", "DESYNC", "ANOMALY", "OK"]
    kraken_age_seconds: float | None
    binance_age_seconds: float | None
    exchange_skew_seconds: float | None


def event_time_to_monotonic(
    event_timestamp,
    *,
    now_epoch: float | None = None,
    now_monotonic: float | None = None,
) -> float:
    if isinstance(event_timestamp, bool):
        raise ValueError("market event timestamp must be numeric or ISO-8601")

    if isinstance(event_timestamp, (int, float)):
        event_epoch = float(event_timestamp)
        if not math.isfinite(event_epoch):
            raise ValueError("market event timestamp must be finite")
        if event_epoch > 100_000_000_000:
            event_epoch /= 1000
    elif isinstance(event_timestamp, str):
        try:
            parsed = datetime.fromisoformat(event_timestamp.replace("Z", "+00:00"))
        except ValueError as error:
            raise ValueError("market event timestamp must be valid ISO-8601") from error
        if parsed.tzinfo is None:
            parsed = parsed.replace(tzinfo=timezone.utc)
        event_epoch = parsed.timestamp()
    else:
        raise ValueError("market event timestamp must be numeric or ISO-8601")

    wall_now = time.time() if now_epoch is None else now_epoch
    monotonic_now = time.monotonic() if now_monotonic is None else now_monotonic
    age_seconds = wall_now - event_epoch
    if age_seconds < -5:
        raise ValueError("market event timestamp is too far in the future")
    return monotonic_now - age_seconds


def assess_market_data(
    kraken_updated_at: float | None,
    binance_updated_at: float | None,
    *,
    now: float | None = None,
    stale_after_seconds: float,
    max_skew_seconds: float,
) -> MarketDataHealth:
    if kraken_updated_at is None or binance_updated_at is None:
        return MarketDataHealth("WAITING", None, None, None)

    current_time = time.monotonic() if now is None else now
    kraken_age = max(0.0, current_time - kraken_updated_at)
    binance_age = max(0.0, current_time - binance_updated_at)
    skew = abs(kraken_updated_at - binance_updated_at)

    if kraken_age > stale_after_seconds or binance_age > stale_after_seconds:
        status = "STALE"
    elif skew > max_skew_seconds:
        status = "DESYNC"
    else:
        status = "OK"

    return MarketDataHealth(status, kraken_age, binance_age, skew)


def assess_market_data_snapshot(
    snapshot_generation: tuple[int | None, int | None],
    current_generation: tuple[int | None, int | None],
    kraken_updated_at: float | None,
    binance_updated_at: float | None,
    *,
    now: float | None = None,
    stale_after_seconds: float,
    max_skew_seconds: float,
) -> MarketDataHealth | None:
    if snapshot_generation != current_generation:
        return None
    return assess_market_data(
        kraken_updated_at,
        binance_updated_at,
        now=now,
        stale_after_seconds=stale_after_seconds,
        max_skew_seconds=max_skew_seconds,
    )
