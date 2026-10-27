import csv
import io
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path

from market_history import MarketStore


class MarketStoreTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.store = MarketStore(Path(self.directory.name) / "history.sqlite3")
        self.now = datetime(2026, 10, 25, 12, tzinfo=timezone.utc)

    def tearDown(self):
        # Close SQLite connections to avoid file locks on Windows
        try:
            if hasattr(self.store, "_pool"):
                self.store._pool.closeall()
        except Exception:
            pass

    def test_market_history_persists_and_groups_day_week_and_month(self):
        self.store.record_tick(
            "BTC/USDT", "Binance", 100, timestamp=self.now - timedelta(minutes=2)
        )
        self.store.record_tick(
            "BTC/USDT", "Binance", 102, timestamp=self.now - timedelta(minutes=1)
        )
        day = self.store.history("BTC/USDT", "Binance", "day", now=self.now)
        self.assertEqual(len(day), 2)
        self.assertEqual(sum(point["ticks"] for point in day), 2)

        self.store.record_tick(
            "BTC/USDT", "Binance", 104, timestamp=self.now - timedelta(minutes=1, seconds=30)
        )
        day = self.store.history("BTC/USDT", "Binance", "day", now=self.now)
        self.assertEqual(len(day), 2)
        self.assertEqual(sorted(point["ticks"] for point in day), [1, 2])
        self.assertAlmostEqual(max(point["high"] for point in day), 104)

        csv_rows = list(csv.DictReader(io.StringIO(
            self.store.export_history("BTC/USDT", "Binance", "day", now=self.now)
        )))
        self.assertEqual(len(csv_rows), 3)
        self.assertEqual(csv_rows[0]["Price_USDT"], "100.0")

    def test_daily_market_summary_uses_first_last_observed_prices(self):
        first = datetime(2026, 10, 25, 0, 5, tzinfo=timezone.utc)
        last = datetime(2026, 10, 25, 23, 50, tzinfo=timezone.utc)
        self.store.record_tick("ETH/USDT", "Kraken", 2000, timestamp=first)
        self.store.record_tick(
            "ETH/USDT", "Kraken", 2100, timestamp=last
        )
        self.store.record_tick(
            "ETH/USDT", "Kraken", 2200, timestamp=datetime(2026, 10, 26, tzinfo=timezone.utc)
        )

        summary = self.store.daily_market_summary(first.date())

        self.assertEqual(len(summary), 1)
        self.assertEqual(summary[0]["open"], 2000)
        self.assertEqual(summary[0]["close"], 2100)
        self.assertEqual(summary[0]["high"], 2100)
        self.assertEqual(summary[0]["ticks"], 2)

    def test_alerts_trigger_on_crossing_once_and_rearm_after_return(self):
        alert = self.store.create_alert("BTC/USDT", "Binance", "CHF", "above", 70)
        market = {
            "symbols": {
                "BTC/USDT": {
                    "Binance": {"price": 100, "status": "live"},
                    "Kraken": {"price": 200, "status": "live"},
                }
            }
        }
        fx = {"rates": {"USD": 2, "CHF": 1.5}}

        triggered = self.store.check_alerts(market, fx)
        self.assertEqual(len(triggered), 1)
        self.assertEqual(triggered[0]["exchange"], "Binance")
        self.assertEqual(triggered[0]["price"], 75)
        self.assertEqual(self.store.check_alerts(market, fx), [])

        market["symbols"]["BTC/USDT"]["Binance"]["price"] = 80
        self.assertEqual(self.store.check_alerts(market, fx), [])
        market["symbols"]["BTC/USDT"]["Binance"]["price"] = 100
        self.assertEqual(len(self.store.check_alerts(market, fx)), 1)
        self.assertEqual(self.store.list_alerts()[0]["id"], alert["id"])

    def test_alerts_ignore_stale_quotes_and_invalid_configuration(self):
        self.store.create_alert("BTC/USDT", "Binance", "EUR", "below", 100)
        stale_market = {
            "symbols": {
                "BTC/USDT": {"Binance": {"price": 1, "status": "stale"}}
            }
        }
        self.assertEqual(
            self.store.check_alerts(stale_market, {"rates": {"USD": 1}}), []
        )
        with self.assertRaisesRegex(ValueError, "market exchange"):
            self.store.create_alert("BTC/USDT", "Unknown", "EUR", "above", 1)
        with self.assertRaisesRegex(ValueError, "finite and positive"):
            self.store.create_alert("BTC/USDT", "Binance", "EUR", "above", float("nan"))

    def test_alert_delete_and_configured_alert_limit(self):
        alert = self.store.create_alert("BTC/USDT", "Binance", "EUR", "above", 1)
        self.assertTrue(self.store.delete_alert(alert["id"]))
        self.assertFalse(self.store.delete_alert(alert["id"]))
        for _ in range(50):
            self.store.create_alert("BTC/USDT", "Binance", "EUR", "above", 1)
        with self.assertRaisesRegex(ValueError, "at most 50"):
            self.store.create_alert("BTC/USDT", "Binance", "EUR", "above", 1)

    def test_source_observations_are_deduplicated_and_queryable_by_event_date(self):
        row = {
            "point": "Interconnector",
            "updated_at": "2026-10-25T10:00:00Z",
            "value": 12,
            "unit": "MWh",
            "source_url": "https://transparency.entsog.eu/",
        }
        first = self.store.record_source_events(
            "energy_flow_observation", "ENTSOG", [row], retrieved_at=self.now
        )
        duplicate = self.store.record_source_events(
            "energy_flow_observation", "ENTSOG", [row], retrieved_at=self.now
        )
        events = self.store.source_events(
            "energy_flow_observation", self.now.date()
        )
        self.assertEqual(first, 1)
        self.assertEqual(duplicate, 0)
        self.assertEqual(events[0]["payload"]["point"], "Interconnector")
        self.assertEqual(events[0]["event_at"], "2026-10-25T10:00:00Z")


if __name__ == "__main__":
    unittest.main()
