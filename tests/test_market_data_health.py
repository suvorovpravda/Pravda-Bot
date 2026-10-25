import unittest

from market_data_health import assess_market_data


class MarketDataHealthTests(unittest.TestCase):
    def assess(self, kraken, binance, now=100):
        return assess_market_data(
            kraken,
            binance,
            now=now,
            stale_after_seconds=15,
            max_skew_seconds=3,
        )

    def test_waits_for_both_exchange_timestamps(self):
        result = self.assess(None, 99)
        self.assertEqual(result.status, "WAITING")

    def test_reports_stale_exchange_data(self):
        result = self.assess(80, 99)
        self.assertEqual(result.status, "STALE")
        self.assertEqual(result.kraken_age_seconds, 20)

    def test_reports_exchange_desynchronization(self):
        result = self.assess(95, 99)
        self.assertEqual(result.status, "DESYNC")
        self.assertEqual(result.exchange_skew_seconds, 4)

    def test_accepts_fresh_synchronized_data(self):
        result = self.assess(98, 99)
        self.assertEqual(result.status, "OK")


if __name__ == "__main__":
    unittest.main()
