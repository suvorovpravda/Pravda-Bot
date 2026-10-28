import unittest

from market_data_health import (
    assess_market_data,
    assess_market_data_snapshot,
    event_time_to_monotonic,
)


class MarketDataHealthTests(unittest.TestCase):
    def assess(self, kraken, binance, now=100):
        return assess_market_data(
            kraken,
            binance,
            now=now,
            stale_after_seconds=4,
            max_skew_seconds=3,
        )

    def test_waits_for_both_exchange_timestamps(self):
        result = self.assess(None, 99)
        self.assertEqual(result.status, "WAITING")

    def test_reports_stale_exchange_data(self):
        result = self.assess(95, 99)
        self.assertEqual(result.status, "STALE")
        self.assertEqual(result.kraken_age_seconds, 5)

    def test_accepts_data_at_stale_age_limit(self):
        result = self.assess(96, 99)
        self.assertEqual(result.status, "OK")
        self.assertEqual(result.kraken_age_seconds, 4)

    def test_reports_exchange_desynchronization(self):
        result = self.assess(100, 96)
        self.assertEqual(result.status, "DESYNC")
        self.assertEqual(result.exchange_skew_seconds, 4)

    def test_accepts_fresh_synchronized_data(self):
        result = self.assess(98, 99)
        self.assertEqual(result.status, "OK")

    def test_snapshot_is_rejected_if_a_price_updated_during_await(self):
        result = assess_market_data_snapshot(
            (1, 1),
            (2, 1),
            98,
            99,
            now=100,
            stale_after_seconds=4,
            max_skew_seconds=3,
        )
        self.assertIsNone(result)

    def test_snapshot_is_rechecked_for_freshness_after_await(self):
        result = assess_market_data_snapshot(
            (1, 1),
            (1, 1),
            95,
            99,
            now=100,
            stale_after_seconds=4,
            max_skew_seconds=3,
        )
        self.assertEqual(result.status, "STALE")

    def test_converts_binance_millisecond_event_time_to_monotonic_age(self):
        result = event_time_to_monotonic(
            1_700_000_000_000,
            now_epoch=1_700_000_005,
            now_monotonic=500,
        )
        self.assertEqual(result, 495)

    def test_converts_kraken_iso_event_time_to_monotonic_age(self):
        result = event_time_to_monotonic(
            "2023-11-14T22:13:20Z",
            now_epoch=1_700_000_005,
            now_monotonic=500,
        )
        self.assertEqual(result, 495)

    def test_rejects_invalid_or_far_future_event_timestamps(self):
        for value in ("not-a-timestamp", 1_700_000_020_000):
            with self.subTest(value=value), self.assertRaises(ValueError):
                event_time_to_monotonic(
                    value,
                    now_epoch=1_700_000_000,
                    now_monotonic=500,
                )


if __name__ == "__main__":
    unittest.main()
