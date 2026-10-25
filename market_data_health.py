from dataclasses import dataclass
import time
from typing import Literal


@dataclass(frozen=True)
class MarketDataHealth:
    status: Literal["WAITING", "STALE", "DESYNC", "OK"]
    kraken_age_seconds: float | None
    binance_age_seconds: float | None
    exchange_skew_seconds: float | None


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
