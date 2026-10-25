import csv
import json
import threading
import unittest
from datetime import datetime, timezone
from functools import partial
from http.server import ThreadingHTTPServer
from pathlib import Path
from tempfile import TemporaryDirectory
from urllib.error import HTTPError
from urllib.request import urlopen

from dashboard import DashboardHandler, read_dashboard_data


class DashboardTests(unittest.TestCase):
    def setUp(self):
        self.temporary_directory = TemporaryDirectory()
        self.addCleanup(self.temporary_directory.cleanup)
        self.csv_path = Path(self.temporary_directory.name) / "observations.csv"

    def write_rows(self, rows):
        fields = [
            "Timestamp_UTC",
            "Symbol",
            "Kraken_Price",
            "Binance_Price",
            "Gross_Spread_Percent",
            "Threshold_Reached",
            "Market_Data_Status",
        ]
        with self.csv_path.open("w", encoding="utf-8", newline="") as target:
            writer = csv.DictWriter(target, fieldnames=fields)
            writer.writeheader()
            writer.writerows(rows)

    def serve(self):
        handler = partial(
            DashboardHandler,
            csv_path=self.csv_path,
            threshold=0.25,
            stale_after_seconds=30,
        )
        server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(server.server_close)
        self.addCleanup(thread.join, 2)
        self.addCleanup(server.shutdown)
        return f"http://127.0.0.1:{server.server_port}"

    def test_reports_freshness_and_returns_latest_twenty_rows(self):
        now = datetime(2026, 1, 1, tzinfo=timezone.utc)
        rows = [
            {
                "Timestamp_UTC": f"2026-01-01T00:00:{second:02d}Z",
                "Symbol": f"PAIR-{second}",
            }
            for second in range(25)
        ]
        self.write_rows(rows)

        payload = read_dashboard_data(
            self.csv_path,
            stale_after_seconds=30,
            now=now,
        )

        self.assertEqual(payload["status"], "Données récentes")
        self.assertEqual(len(payload["observations"]), 20)
        self.assertEqual(payload["observations"][0]["Symbol"], "PAIR-24")
        self.assertEqual(payload["observations"][-1]["Symbol"], "PAIR-5")

    def test_empty_or_invalid_csv_timestamp_does_not_claim_bot_is_active(self):
        self.write_rows([])
        empty = read_dashboard_data(self.csv_path, stale_after_seconds=30)
        self.assertEqual(empty["status"], "Aucune observation")

        self.write_rows([{"Timestamp_UTC": "2025-12-31T23:59:00Z", "Symbol": "BTC/USDT"}])
        stale = read_dashboard_data(
            self.csv_path,
            stale_after_seconds=30,
            now=datetime(2026, 1, 1, tzinfo=timezone.utc),
        )
        self.assertEqual(stale["status"], "Données obsolètes")

        self.write_rows([{"Timestamp_UTC": "invalid", "Symbol": "BTC/USDT"}])
        invalid = read_dashboard_data(self.csv_path, stale_after_seconds=30)
        self.assertEqual(invalid["status"], "État inconnu")

    def test_data_endpoint_uses_csv_schema_and_returns_threshold(self):
        self.write_rows(
            [
                {
                    "Timestamp_UTC": "2026-01-01T00:00:00Z",
                    "Symbol": "BTC/USDT",
                    "Gross_Spread_Percent": "0.4",
                    "Market_Data_Status": "OK",
                }
            ]
        )
        base_url = self.serve()

        with urlopen(f"{base_url}/data") as response:
            payload = json.load(response)

        self.assertEqual(payload["threshold"], 0.25)
        self.assertEqual(payload["observations"][0]["Gross_Spread_Percent"], "0.4")
        self.assertEqual(response.headers["Cache-Control"], "no-store")

    def test_homepage_renders_csv_values_as_text_not_html(self):
        base_url = self.serve()

        with urlopen(base_url) as response:
            html = response.read().decode("utf-8")

        self.assertIn("textContent = row[key]", html)
        self.assertNotIn("innerHTML", html)

    def test_unknown_path_returns_not_found(self):
        base_url = self.serve()

        with self.assertRaises(HTTPError) as error:
            urlopen(f"{base_url}/unknown")

        self.assertEqual(error.exception.code, 404)


if __name__ == "__main__":
    unittest.main()
