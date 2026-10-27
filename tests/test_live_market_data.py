import json
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path
from unittest.mock import Mock, patch

import live_market_data


class LiveMarketDataTests(unittest.TestCase):
    def test_snapshot_publishes_exchange_quotes_and_reader_marks_freshness(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "snapshot.json"
            now = datetime(2026, 10, 25, 12, tzinfo=timezone.utc)
            with patch.object(live_market_data, "_last_snapshot_write", 0):
                written = live_market_data.write_market_snapshot(
                    {"Binance": {"BTC/USDT": 100.0}, "Kraken": {"BTC/USDT": 101.0}},
                    {"Binance": {"BTC/USDT": 99.0}, "Kraken": {"BTC/USDT": 98.0}},
                    path=path,
                    now_monotonic=100.0,
                    now_utc=now,
                )
            result = live_market_data.read_market_snapshot(path, now=now)

        self.assertTrue(written)
        self.assertEqual(result["status"], "live")
        self.assertEqual(result["symbols"]["BTC/USDT"]["Binance"]["price"], 100.0)
        self.assertEqual(
            result["symbols"]["BTC/USDT"]["Kraken"]["received_at"],
            (now - timedelta(seconds=2))
            .isoformat(timespec="milliseconds")
            .replace("+00:00", "Z"),
        )

    def test_old_quotes_are_marked_stale(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "snapshot.json"
            received = datetime(2026, 10, 25, 11, 59, 40, tzinfo=timezone.utc)
            path.write_text(
                json.dumps(
                    {
                        "symbols": {
                            "ETH/USDT": {
                                "Binance": {
                                    "price": 3000,
                                    "received_at": received.isoformat(),
                                }
                            }
                        }
                    }
                ),
                encoding="utf-8",
            )
            now = received + timedelta(seconds=30)
            result = live_market_data.read_market_snapshot(
                path, stale_after_seconds=10, now=now
            )

        self.assertEqual(result["status"], "stale")
        self.assertEqual(result["symbols"]["ETH/USDT"]["Binance"]["status"], "stale")

    def test_reference_fx_rates_are_validated_and_cached(self):
        records = [
            {"date": "2026-10-23", "base": "EUR", "quote": quote, "rate": 1.25}
            for quote in live_market_data.FOREX_QUOTES
        ]
        response = Mock()
        response.json.return_value = records
        response.raise_for_status.return_value = None
        with patch.object(live_market_data, "_forex_cache", None), patch.object(
            live_market_data, "_forex_cache_updated_at", 0
        ), patch("live_market_data.requests.get", return_value=response) as request:
            now = datetime(2026, 10, 25, tzinfo=timezone.utc)
            result = live_market_data.get_reference_fx_rates(now=now)
            cached = live_market_data.get_reference_fx_rates(now=now)

        request.assert_called_once()
        self.assertEqual(result, cached)
        self.assertIsNot(result, cached)
        self.assertEqual(result["date"], "2026-10-23")
        self.assertEqual(result["rates"]["USD"], 1.25)
        self.assertEqual(result["freshness"], "delayed")
        self.assertEqual(result["source_age_days"], 2)

    def test_cached_reference_rate_becomes_stale_as_source_date_ages(self):
        records = [
            {"date": "2026-10-23", "base": "EUR", "quote": quote, "rate": 1.25}
            for quote in live_market_data.FOREX_QUOTES
        ]
        response = Mock()
        response.json.return_value = records
        response.raise_for_status.return_value = None
        with patch.object(live_market_data, "_forex_cache", None), patch.object(
            live_market_data, "_forex_cache_updated_at", 0
        ), patch("live_market_data.requests.get", return_value=response):
            live_market_data.get_reference_fx_rates(
                now=datetime(2026, 10, 25, tzinfo=timezone.utc)
            )
            stale = live_market_data.get_reference_fx_rates(
                now=datetime(2026, 10, 30, tzinfo=timezone.utc)
            )
        self.assertEqual(stale["freshness"], "stale")
        self.assertEqual(stale["source_age_days"], 7)


if __name__ == "__main__":
    unittest.main()
