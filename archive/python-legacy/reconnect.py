import asyncio
import logging
import math
import random
import time
from collections.abc import Awaitable, Callable


logger = logging.getLogger("PravdaWebSocket")


class ExponentialBackoff:
    def __init__(
        self,
        base_delay: float = 1.0,
        max_delay: float = 60.0,
        factor: float = 2.0,
        jitter: bool = True,
    ) -> None:
        if not math.isfinite(base_delay) or base_delay <= 0:
            raise ValueError("base_delay must be finite and greater than zero")
        if not math.isfinite(max_delay) or max_delay < base_delay:
            raise ValueError("max_delay must be finite and at least base_delay")
        if not math.isfinite(factor) or factor <= 1:
            raise ValueError("factor must be finite and greater than one")

        self.base_delay = base_delay
        self.max_delay = max_delay
        self.factor = factor
        self.jitter = jitter
        self._next_delay = base_delay

    def get_delay(self) -> float:
        delay = self._next_delay
        self._next_delay = min(self.max_delay, self._next_delay * self.factor)
        if self.jitter:
            delay = min(self.max_delay, delay * random.uniform(0.8, 1.2))
        return delay

    def reset(self) -> None:
        self._next_delay = self.base_delay


async def run_with_reconnect(
    feed: Callable[[], Awaitable[None]],
    feed_name: str,
    *,
    backoff: ExponentialBackoff | None = None,
    stable_connection_seconds: float = 30.0,
    sleep: Callable[[float], Awaitable[None]] = asyncio.sleep,
) -> None:
    if not math.isfinite(stable_connection_seconds) or stable_connection_seconds < 0:
        raise ValueError("stable_connection_seconds must be finite and non-negative")
    retry_backoff = backoff or ExponentialBackoff(base_delay=1.0, max_delay=30.0)

    while True:
        started_at = time.monotonic()
        try:
            await feed()
            failure = "stream ended"
        except asyncio.CancelledError:
            raise
        except Exception as error:
            failure = type(error).__name__

        if time.monotonic() - started_at >= stable_connection_seconds:
            retry_backoff.reset()

        delay = retry_backoff.get_delay()
        logger.warning(
            "%s disconnected (%s); reconnecting in %.2f seconds",
            feed_name,
            failure,
            delay,
        )
        await sleep(delay)
