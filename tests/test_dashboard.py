import csv
import html
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
from unittest.mock import Mock, patch

import psutil

import dashboard
from dashboard import DashboardHandler, HTML_TEMPLATE, read_dashboard_data


class DashboardTests(unittest.TestCase):
    def setUp(self):
        self.temporary_directory = TemporaryDirectory()
        self.addCleanup(self.temporary_directory.cleanup)
        self.csv_path = Path(self.temporary_directory.name) / "observations.csv"
        self.lng_csv_path = Path(self.temporary_directory.name) / "lng_observations.csv"

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

    def test_system_metrics_reuses_process_and_initializes_cpu_sample_once(self):
        original_process = dashboard.SYSTEM_METRICS_PROCESS
        dashboard.SYSTEM_METRICS_PROCESS = None
        self.addCleanup(setattr, dashboard, "SYSTEM_METRICS_PROCESS", original_process)

        process = Mock()
        process.memory_info.return_value.rss = 1024 * 1024
        process.cpu_percent.side_effect = [0.0, 12.5, 24.0]
        with patch("dashboard.psutil.Process", return_value=process) as process_factory:
            first = dashboard.get_system_metrics()
            second = dashboard.get_system_metrics()

        process_factory.assert_called_once()
        self.assertEqual(first["cpu_percent"], 12.5)
        self.assertEqual(second["cpu_percent"], 24.0)
        self.assertEqual(process.cpu_percent.call_count, 3)

    def serve(self):
        handler = partial(
            DashboardHandler,
            csv_path=self.csv_path,
            lng_csv_path=self.lng_csv_path,
            market_snapshot_path=Path(self.temporary_directory.name) / "market_snapshot.json",
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

    def test_metrics_endpoint_returns_dashboard_process_metrics(self):
        base_url = self.serve()

        with urlopen(f"{base_url}/api/metrics") as response:
            payload = json.load(response)

        self.assertEqual(response.status, 200)
        self.assertEqual(payload["status"], "success")
        self.assertEqual(
            set(payload["system"]),
            {"memory_rss_mb", "cpu_percent", "uptime_seconds"},
        )
        self.assertGreaterEqual(payload["system"]["memory_rss_mb"], 0)
        self.assertGreaterEqual(payload["system"]["cpu_percent"], 0)
        self.assertGreaterEqual(payload["system"]["uptime_seconds"], 0)
        self.assertIsInstance(payload["timestamp"], (int, float))
        self.assertEqual(response.headers["Cache-Control"], "no-store")

    def test_metrics_endpoint_returns_service_unavailable_on_collection_error(self):
        base_url = self.serve()

        with patch("dashboard.get_system_metrics", side_effect=psutil.AccessDenied):
            with self.assertRaises(HTTPError) as error:
                urlopen(f"{base_url}/api/metrics")

        self.assertEqual(error.exception.code, 503)
        payload = json.load(error.exception)
        self.assertEqual(payload["status"], "error")
        self.assertEqual(payload["error"], "System metrics unavailable")

    def test_missing_csv_files_return_empty_success_payloads(self):
        base_url = self.serve()

        with urlopen(f"{base_url}/data") as response:
            crypto_payload = json.load(response)
            self.assertEqual(response.status, 200)
        with urlopen(f"{base_url}/lng-data") as response:
            lng_payload = json.load(response)
            self.assertEqual(response.status, 200)

        self.assertEqual(crypto_payload["observations"], [])
        self.assertEqual(crypto_payload["status"], "Aucune observation")
        self.assertEqual(lng_payload["observations"], [])
        self.assertEqual(lng_payload["status"], "Aucune observation GNL")

    def test_homepage_renders_csv_values_as_text_not_html(self):
        base_url = self.serve()

        with urlopen(base_url) as response:
            html = response.read().decode("utf-8")
            self.assertEqual(response.headers["Cache-Control"], "no-store")
            self.assertEqual(response.headers["Content-Type"], "text/html; charset=utf-8")

        self.assertIn("textContent = safeText(value)", html)
        self.assertNotIn("innerHTML", html)
        self.assertIn('target="_blank" rel="noopener noreferrer"', html)
        self.assertIn("function drawMap(rows)", html)
        self.assertIn("/api/eu/flows", html)
        self.assertIn("/api/geopolitics", html)

    def test_unified_dashboard_template_has_crypto_and_lng_sections(self):
        template_text = html.unescape(HTML_TEMPLATE)
        self.assertIn("PRAVDA", template_text)
        self.assertIn("Gaz, flux & risques", template_text)
        self.assertIn("Réseau gazier européen", template_text)
        self.assertIn("Actualités & risques géopolitiques", template_text)
        self.assertIn("Aucun cours live configuré", template_text)

    def test_european_flows_endpoint_serves_official_payload(self):
        expected = {
            "status": "live",
            "observations": [{"point": "Border Point"}],
        }
        with patch("dashboard.get_entsog_flows", return_value=expected):
            base_url = self.serve()
            with urlopen(f"{base_url}/api/eu/flows") as response:
                payload = json.load(response)

        self.assertEqual(response.status, 200)
        self.assertEqual(payload, expected)

    def test_gas_price_endpoint_does_not_claim_an_unconfigured_live_quote(self):
        base_url = self.serve()
        with urlopen(f"{base_url}/api/gas-price") as response:
            payload = json.load(response)

        self.assertEqual(response.status, 200)
        self.assertEqual(payload["status"], "unavailable")
        self.assertIsNone(payload["value"])

    def test_market_endpoint_reads_local_exchange_snapshot(self):
        snapshot = {
            "status": "live",
            "source": "Kraken + Binance public ticker streams",
            "symbols": {"BTC/USDT": {"Binance": {"price": 90000, "status": "live"}}},
        }
        with patch("dashboard.read_market_snapshot", return_value=snapshot):
            base_url = self.serve()
            with urlopen(f"{base_url}/api/market") as response:
                payload = json.load(response)

        self.assertEqual(response.status, 200)
        self.assertEqual(payload, snapshot)

    def test_fx_endpoint_returns_daily_reference_rates_and_cadence_notice(self):
        rates = {
            "status": "available",
            "date": "2026-10-23",
            "rates": {"USD": 1.12},
            "notice": "Daily reference rates",
        }
        with patch("dashboard.get_reference_fx_rates", return_value=rates):
            base_url = self.serve()
            with urlopen(f"{base_url}/api/fx") as response:
                payload = json.load(response)

        self.assertEqual(response.status, 200)
        self.assertEqual(payload["date"], "2026-10-23")
        self.assertIn("Daily", payload["notice"])

    def test_geopolitical_feed_errors_are_reported_as_unavailable(self):
        from energy_data import EnergyDataError

        with patch("dashboard.get_geopolitical_news", side_effect=EnergyDataError("offline")):
            base_url = self.serve()
            with self.assertRaises(HTTPError) as error:
                urlopen(f"{base_url}/api/geopolitics")

        self.assertEqual(error.exception.code, 502)
        payload = json.load(error.exception)
        self.assertEqual(payload["status"], "unavailable")

    def test_lng_data_endpoint_returns_latest_rows_and_labels_samples(self):
        rows = [
            {
                "Timestamp_UTC": f"2026-01-01T00:00:{second:02d}Z",
                "Benchmark": f"BENCHMARK-{second}",
                "Price_USD": str(second + 1),
                "Status": "SIMULATED",
                "Note": "<script>alert(1)</script>",
            }
            for second in range(12)
        ]
        with self.lng_csv_path.open("w", encoding="utf-8", newline="") as target:
            writer = csv.DictWriter(
                target,
                fieldnames=[
                    "Timestamp_UTC",
                    "Benchmark",
                    "Price_USD",
                    "Status",
                    "Note",
                ],
            )
            writer.writeheader()
            writer.writerows(rows)
        base_url = self.serve()

        with urlopen(f"{base_url}/lng-data") as response:
            payload = json.load(response)

        self.assertEqual(payload["status"], "Exemples simulés (pas de données de marché réelles)")
        self.assertEqual(len(payload["observations"]), 10)
        self.assertEqual(payload["observations"][0]["Benchmark"], "BENCHMARK-11")
        self.assertEqual(
            payload["observations"][-1]["Note"],
            "<script>alert(1)</script>",
        )
        self.assertEqual(response.headers["Cache-Control"], "no-store")

    def test_lng_data_endpoint_rejects_incompatible_csv_schema(self):
        self.lng_csv_path.write_text("Timestamp,Status\nnow,SIMULATED\n", encoding="utf-8")
        base_url = self.serve()

        with self.assertRaises(HTTPError) as error:
            urlopen(f"{base_url}/lng-data")

        self.assertEqual(error.exception.code, 500)

    def test_unknown_path_returns_not_found(self):
        base_url = self.serve()

        with self.assertRaises(HTTPError) as error:
            urlopen(f"{base_url}/unknown")

        self.assertEqual(error.exception.code, 404)


if __name__ == "__main__":
    unittest.main()
