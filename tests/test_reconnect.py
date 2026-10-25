import asyncio
import unittest
from unittest.mock import patch

from reconnect import ExponentialBackoff, run_with_reconnect


class ExponentialBackoffTests(unittest.TestCase):
    def test_delay_grows_exponentially_and_caps_at_maximum(self):
        backoff = ExponentialBackoff(
            base_delay=1,
            max_delay=5,
            factor=2,
            jitter=False,
        )

        self.assertEqual(
            [backoff.get_delay() for _ in range(5)],
            [1, 2, 4, 5, 5],
        )

    def test_jitter_is_applied_without_exceeding_maximum(self):
        backoff = ExponentialBackoff(base_delay=5, max_delay=6, jitter=True)
        with patch("reconnect.random.uniform", return_value=1.2):
            self.assertEqual(backoff.get_delay(), 6)

    def test_reset_restarts_backoff_sequence(self):
        backoff = ExponentialBackoff(base_delay=1, max_delay=10, jitter=False)
        self.assertEqual(backoff.get_delay(), 1)
        self.assertEqual(backoff.get_delay(), 2)
        backoff.reset()
        self.assertEqual(backoff.get_delay(), 1)

    def test_invalid_backoff_settings_are_rejected(self):
        for settings in (
            {"base_delay": 0},
            {"base_delay": 2, "max_delay": 1},
            {"factor": 1},
        ):
            with self.subTest(settings=settings), self.assertRaises(ValueError):
                ExponentialBackoff(**settings)


class RunWithReconnectTests(unittest.IsolatedAsyncioTestCase):
    async def test_retries_failures_with_backoff_and_propagates_cancellation(self):
        backoff = ExponentialBackoff(
            base_delay=1,
            max_delay=10,
            factor=2,
            jitter=False,
        )
        delays = []
        feed_calls = 0

        async def feed():
            nonlocal feed_calls
            feed_calls += 1
            raise OSError("network unavailable")

        async def sleep(delay):
            delays.append(delay)
            if len(delays) == 3:
                raise asyncio.CancelledError

        with self.assertLogs("PravdaWebSocket", level="WARNING"):
            with self.assertRaises(asyncio.CancelledError):
                await run_with_reconnect(
                    feed,
                    "test-feed",
                    backoff=backoff,
                    stable_connection_seconds=100,
                    sleep=sleep,
                )

        self.assertEqual(feed_calls, 3)
        self.assertEqual(delays, [1, 2, 4])

    async def test_stable_session_resets_retry_delay(self):
        backoff = ExponentialBackoff(
            base_delay=1,
            max_delay=10,
            factor=2,
            jitter=False,
        )
        delays = []
        monotonic_values = iter((0, 1, 2, 33, 34, 35))
        feed_calls = 0

        async def feed():
            nonlocal feed_calls
            feed_calls += 1
            raise OSError("network unavailable")

        async def sleep(delay):
            delays.append(delay)
            if len(delays) == 3:
                raise asyncio.CancelledError

        with patch("reconnect.time.monotonic", side_effect=lambda: next(monotonic_values)):
            with self.assertLogs("PravdaWebSocket", level="WARNING"):
                with self.assertRaises(asyncio.CancelledError):
                    await run_with_reconnect(
                        feed,
                        "test-feed",
                        backoff=backoff,
                        stable_connection_seconds=30,
                        sleep=sleep,
                    )

        self.assertEqual(feed_calls, 3)
        self.assertEqual(delays, [1, 1, 2])

    async def test_normal_stream_end_also_triggers_reconnect(self):
        backoff = ExponentialBackoff(
            base_delay=1,
            max_delay=10,
            jitter=False,
        )
        delays = []

        async def feed():
            return None

        async def sleep(delay):
            delays.append(delay)
            raise asyncio.CancelledError

        with self.assertLogs("PravdaWebSocket", level="WARNING"):
            with self.assertRaises(asyncio.CancelledError):
                await run_with_reconnect(
                    feed,
                    "test-feed",
                    backoff=backoff,
                    stable_connection_seconds=100,
                    sleep=sleep,
                )

        self.assertEqual(delays, [1])


if __name__ == "__main__":
    unittest.main()
