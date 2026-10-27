import argparse
import base64
import csv
import hashlib
import html
import io
import json
import math
import re
import socket
import threading
import unittest
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
from functools import partial
from http.server import ThreadingHTTPServer
from pathlib import Path
from tempfile import TemporaryDirectory
from urllib.error import HTTPError
from urllib.request import ProxyHandler, Request, build_opener
from urllib.parse import urlsplit
from unittest.mock import Mock, patch

import psutil

import dashboard
from dashboard import DashboardHandler, HTML_TEMPLATE, read_dashboard_data
from market_history import MarketStore

urlopen = build_opener(ProxyHandler({})).open


class DashboardTests(unittest.TestCase):
    def setUp(self):
        self.temporary_directory = TemporaryDirectory()
        self.addCleanup(self.temporary_directory.cleanup)
        self.csv_path = Path(self.temporary_directory.name) / "observations.csv"
        self.lng_csv_path = Path(self.temporary_directory.name) / "lng_observations.csv"
        self.history_store = MarketStore(
            Path(self.temporary_directory.name) / "history.sqlite3"
        )
        self.quiet_http_logs = patch.object(DashboardHandler, "log_message")
        self.quiet_http_logs.start()
        self.addCleanup(self.quiet_http_logs.stop)

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

    def serve(self, **options):
        handler = partial(
            DashboardHandler,
            csv_path=self.csv_path,
            lng_csv_path=self.lng_csv_path,
            market_snapshot_path=Path(self.temporary_directory.name) / "market_snapshot.json",
            market_history_store=self.history_store,
            threshold=0.25,
            stale_after_seconds=30,
            **options,
        )
        server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(server.server_close)
        self.addCleanup(thread.join, 2)
        self.addCleanup(server.shutdown)
        return f"http://127.0.0.1:{server.server_port}"

    def raw_request(self, base_url, request):
        with socket.create_connection(("127.0.0.1", urlsplit(base_url).port), timeout=2) as client:
            client.sendall(request)
            client.shutdown(socket.SHUT_WR)
            chunks = []
            while True:
                try:
                    chunk = client.recv(65536)
                except ConnectionResetError:
                    break
                if not chunk:
                    break
                chunks.append(chunk)
        response = b"".join(chunks)
        self.assertTrue(response, "Expected a bounded HTTP error response")
        return int(response.split(b" ", 2)[1]), response

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
        process = Mock()
        process.memory_info.return_value.rss = 5 * 1024 * 1024
        process.cpu_percent.side_effect = [0.0, 12.5]
        with patch.object(dashboard, "SYSTEM_METRICS_PROCESS", None), patch(
            "dashboard.psutil.Process", return_value=process
        ) as factory:
            with urlopen(f"{base_url}/api/metrics") as response:
                payload = json.load(response)
            factory.assert_called_once()

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

    def test_history_endpoint_and_csv_export_read_persistent_store(self):
        now = datetime.now(timezone.utc)
        self.history_store.record_tick("BTC/USDT", "Binance", 100, timestamp=now)
        base_url = self.serve()

        with urlopen(
            f"{base_url}/api/market/history?symbol=BTC%2FUSDT&exchange=Binance&period=day"
        ) as response:
            history = json.load(response)
        with urlopen(
            f"{base_url}/api/market/history.csv?symbol=BTC%2FUSDT&exchange=Binance&period=day"
        ) as response:
            exported = response.read().decode("utf-8")
            self.assertEqual(response.headers["Content-Type"], "text/csv; charset=utf-8")

        self.assertEqual(history["observations"][0]["price"], 100)
        self.assertIn("Price_USDT", exported)
        self.assertIn("100.0", exported)

    def test_alert_api_persists_and_rejects_cross_origin_updates(self):
        base_url = self.serve()
        request = Request(
            f"{base_url}/api/alerts",
            data=json.dumps(
                {
                    "symbol": "BTC/USDT",
                    "exchange": "Kraken",
                    "currency": "CHF",
                    "direction": "above",
                    "target": 50000,
                }
            ).encode("utf-8"),
            headers={
                "Content-Type": "application/json",
                "Origin": base_url,
            },
            method="POST",
        )
        with urlopen(request) as response:
            created = json.load(response)
            self.assertEqual(response.status, 201)
        self.assertEqual(created["alert"]["exchange"], "Kraken")

        with urlopen(f"{base_url}/api/alerts") as response:
            alerts = json.load(response)["alerts"]
        self.assertEqual(len(alerts), 1)
        self.assertEqual(alerts[0]["currency"], "CHF")

        cross_origin = Request(
            f"{base_url}/api/alerts",
            data=b'{"symbol":"BTC/USDT","exchange":"Binance","currency":"EUR","direction":"above","target":1}',
            headers={
                "Content-Type": "application/json",
                "Origin": "https://attacker.invalid",
            },
            method="POST",
        )
        with self.assertRaises(HTTPError) as error:
            urlopen(cross_origin)
        self.assertEqual(error.exception.code, 403)

    def test_daily_report_csv_includes_market_and_source_provenance(self):
        report_date = datetime(2026, 10, 25, tzinfo=timezone.utc).date()
        self.history_store.record_tick(
            "BTC/USDT", "Binance", 100,
            timestamp=datetime(2026, 10, 25, 10, tzinfo=timezone.utc),
        )
        with patch(
            "dashboard.get_entsog_flows",
            return_value={
                "status": "live",
                "source": "ENTSOG",
                "source_url": "https://entsog.example",
                "observations": [
                    {
                        "updated_at": "2026-10-25T11:00:00Z",
                        "direction": "entry",
                        "point": "Test point",
                        "value": 5,
                        "unit": "MWh",
                        "country": "FR",
                        "operator": "Operator",
                    }
                ],
            },
        ), patch(
            "dashboard.get_geopolitical_news",
            return_value={
                "source": "GDELT",
                "articles": [
                    {
                        "published_at": "2026-10-25T12:00:00Z",
                        "source": "Example",
                        "title": "Reported event",
                        "url": "https://news.example/article",
                    }
                ],
            },
        ):
            report = dashboard.daily_report_csv(report_date, store=self.history_store)

        rows = list(csv.DictReader(io.StringIO(report)))
        kinds = {row["record_type"] for row in rows}
        self.assertEqual(
            kinds,
            {"market_summary", "energy_flow_observation", "reported_news"},
        )
        self.assertEqual(rows[0]["source"], "Binance public ticker")
        self.assertTrue(any(row["source_url"] == "https://news.example/article" for row in rows))

    def test_unified_dashboard_template_has_crypto_and_lng_sections(self):
        template_text = html.unescape(HTML_TEMPLATE)
        self.assertIn("PRAVDA", template_text)
        self.assertIn("Marchés, énergie & risques", template_text)
        self.assertIn("Réseau gazier européen", template_text)
        self.assertIn("Actualités & risques géopolitiques", template_text)
        self.assertIn("TTF · aucune cotation vérifiée", template_text)
        self.assertIn("Prix et unité non affichés", template_text)
        self.assertIn('<option>EUR</option><option>CHF</option>', template_text)
        self.assertIn("convertQuote(price, currency)", template_text)
        self.assertNotIn("Contrat futures néerlandais · EUR/MWh", template_text)

    def test_dashboard_language_selector_supports_and_persists_three_languages(self):
        template_text = html.unescape(HTML_TEMPLATE)
        self.assertIn('id="language-select"', template_text)
        for language, display_name in (
            ("fr", "Français"),
            ("ru", "Русский"),
            ("en", "English"),
        ):
            with self.subTest(language=language):
                self.assertIn(f'<option value="{language}">{display_name}</option>', template_text)
                self.assertIn(f"{language}: {{", template_text)
        self.assertIn('localStorage.setItem("pravda-dashboard-language"', template_text)
        self.assertIn('document.documentElement.lang = currentLanguage', template_text)
        self.assertIn('new Intl.NumberFormat(languageLocales[currentLanguage]', template_text)

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

    def test_security_headers_allow_only_the_actual_inline_script_hash(self):
        base_url = self.serve()
        with urlopen(base_url) as response:
            page = response.read().decode("utf-8")
            headers = response.headers
        self.assertEqual(headers["X-Frame-Options"], "DENY")
        self.assertIn("camera=()", headers["Permissions-Policy"])
        self.assertEqual(headers["X-Content-Type-Options"], "nosniff")
        self.assertNotIn("Python", headers["Server"])
        policy = headers["Content-Security-Policy"]
        script_policy = next(part for part in policy.split(";") if part.strip().startswith("script-src"))
        self.assertNotIn("'unsafe-inline'", script_policy)
        scripts = re.findall(r"<script>(.*?)</script>", page, re.S)
        self.assertTrue(scripts)
        self.assertEqual(len(scripts), page.count("<script>"))
        self.assertNotRegex(page, r"[\x00-\x08\x0b\x0c\x0e-\x1f]")
        expected_hashes = set()
        for script in scripts:
            digest = base64.b64encode(hashlib.sha256(script.encode()).digest()).decode()
            expected_hashes.add("'sha256-" + digest + "'")
        self.assertEqual(set(re.findall(r"'sha256-[^']+'", script_policy)), expected_hashes)
        self.assertIn("frame-src https://open.spotify.com", policy)
        with self.assertRaises(HTTPError) as error:
            urlopen(base_url + "/missing-secret-path")
        self.assertEqual(error.exception.headers["X-Frame-Options"], "DENY")
        self.assertEqual(json.load(error.exception), {"status": "error", "error": "Not Found"})

    def test_get_host_origin_and_fetch_site_are_checked_against_the_real_port(self):
        base_url = self.serve()
        port = urlsplit(base_url).port
        for headers in (
            {"Host": "attacker.invalid"}, {"Host": f"127.0.0.1:{port + 1 if port < 65535 else port - 1}"},
            {"Origin": "https://attacker.invalid"}, {"Origin": base_url + "/path"},
            {"Origin": base_url + "?secret=1"}, {"Origin": "null"},
            {"Sec-Fetch-Site": "cross-site"}, {"Sec-Fetch-Site": "same-site"}, {},
        ):
            with self.subTest(headers=headers), self.assertRaises(HTTPError) as error:
                urlopen(Request(base_url + "/api/alerts/check", headers=headers))
            self.assertEqual(error.exception.code, 403)
        for host in ("127.0.0.1", "localhost", "[::1]"):
            origin = f"http://{host}:{port}"
            with urlopen(Request(base_url + "/api/alerts", headers={"Host": f"{host}:{port}", "Origin": origin})) as response:
                self.assertEqual(response.status, 200)
        with patch("dashboard.read_market_snapshot", return_value={"symbols": {}}), patch(
            "dashboard.get_reference_fx_rates", return_value={"rates": {}}
        ):
            with urlopen(Request(base_url + "/api/alerts/check", headers={"Sec-Fetch-Site": "same-origin"})) as response:
                self.assertEqual(response.status, 200)

    def test_mutations_require_explicit_same_origin_and_delete_stays_available(self):
        alert = self.history_store.create_alert("BTC/USDT", "Kraken", "CHF", "above", 50000)
        base_url = self.serve()
        for method, path, data in (("POST", "/api/alerts", b"{}"), ("DELETE", f"/api/alerts/{alert['id']}", None)):
            for origin in (None, "https://attacker.invalid", base_url + "/path", "http://secret@" + urlsplit(base_url).netloc):
                with self.subTest(method=method, origin=origin):
                    headers = {"Content-Type": "application/json"}
                    if origin is not None:
                        headers["Origin"] = origin
                    with self.assertRaises(HTTPError) as error:
                        urlopen(Request(base_url + path, data=data, headers=headers, method=method))
                    self.assertEqual(error.exception.code, 403)
            self.assertEqual(len(self.history_store.list_alerts()), 1)
        with urlopen(Request(base_url + f"/api/alerts/{alert['id']}", headers={"Origin": base_url}, method="DELETE")) as response:
            self.assertEqual(json.load(response)["status"], "deleted")
        self.assertEqual(self.history_store.list_alerts(), [])

    def test_headers_body_and_request_targets_have_small_independent_limits(self):
        base_url = self.serve()
        host = b"Host: " + urlsplit(base_url).netloc.encode() + b"\r\n"
        origin = b"Origin: " + base_url.encode() + b"\r\n"
        cases = [
            (b"GET /api/alerts HTTP/1.1\r\n", b"X-Large: " + b"x" * 8192 + b"\r\n", 431),
            (b"GET /api/alerts HTTP/1.1\r\n", b"".join(f"X-{i}: x\r\n".encode() for i in range(65)), 431),
            (b"GET /api/alerts HTTP/1.1\r\n", b"".join(f"X-{i}: ".encode() + b"x" * 4000 + b"\r\n" for i in range(5)), 431),
            (b"POST /api/alerts HTTP/1.1\r\n", origin + b"Content-Length: 4097\r\n", 413),
            (b"GET /api/alerts HTTP/1.1\r\n", b"Content-Length: 1\r\n", 413),
            (b"GET /api/alerts HTTP/1.1\r\n", b"Content-Length: 0\r\nContent-Length: 0\r\n", 400),
            (b"GET /api/alerts HTTP/1.1\r\n", b"Transfer-Encoding: chunked\r\n", 400),
            (b"GET /api/alerts HTTP/1.1\r\n", host, 403),
            (b"GET /" + b"x" * 2048 + b" HTTP/1.1\r\n", b"", 414),
        ]
        for index, (line, headers, expected) in enumerate(cases):
            with self.subTest(case=index):
                code, _ = self.raw_request(base_url, line + host + headers + b"\r\n")
                self.assertEqual(code, expected)

    def test_socket_timeout_does_not_block_later_requests(self):
        base_url = self.serve()
        with patch.object(DashboardHandler, "connection_timeout_seconds", 0.05):
            with socket.create_connection(("127.0.0.1", urlsplit(base_url).port), timeout=1) as client:
                client.sendall(b"GET /api/alerts HTTP/1.1\r\nHost: ")
                self.assertEqual(client.recv(1), b"")
        with urlopen(base_url + "/api/alerts") as response:
            self.assertEqual(response.status, 200)

    def test_rate_limiter_is_validated_atomic_bounded_and_expires(self):
        for value in ("0", "-1", "1.5", "inf", "10001", True):
            with self.subTest(value=value), self.assertRaises(argparse.ArgumentTypeError):
                dashboard.rate_limit_setting(value)
        limiter = dashboard.RequestRateLimiter(10, max_clients=3)
        with ThreadPoolExecutor(max_workers=16) as executor:
            results = list(executor.map(lambda _: limiter.retry_after("local", now=100), range(100)))
        self.assertEqual(results.count(0), 10)
        self.assertEqual(limiter.retry_after("local", now=159.9), 1)
        self.assertEqual(limiter.retry_after("local", now=160), 0)
        for client in range(20):
            limiter.retry_after(str(client), now=160)
        self.assertLessEqual(len(limiter._clients), 3)

    def test_configured_http_limit_and_default_polling_budget(self):
        base_url = self.serve(rate_limit_per_minute=2)
        for _ in range(2):
            with urlopen(base_url + "/api/alerts") as response:
                self.assertEqual(response.status, 200)
        with self.assertRaises(HTTPError) as error:
            urlopen(base_url + "/api/alerts")
        self.assertEqual(error.exception.code, 429)
        self.assertGreaterEqual(int(error.exception.headers["Retry-After"]), 1)
        periods = dict(re.findall(r"setInterval\((\w+),\s*(\d+)\)", HTML_TEMPLATE))
        self.assertIn("checkPriceAlerts", periods)
        minute_requests = sum(60000 / int(periods[name]) for name in (
            "refreshMarket", "refreshFx", "refreshFlows", "refreshNews", "refreshCrypto", "refreshSources", "checkPriceAlerts"
        ))
        self.assertGreater(minute_requests, 85)
        limiter = dashboard.RequestRateLimiter(dashboard.DEFAULT_RATE_LIMIT_PER_MINUTE)
        for _ in range(math.ceil(minute_requests) + 10):
            self.assertEqual(limiter.retry_after("local", now=100), 0)

    def test_data_endpoint_stays_available_after_50001_rows(self):
        self.write_rows({"Timestamp_UTC": "2026-10-27T00:00:00Z", "Symbol": str(index)}
                        for index in range(50001))
        with urlopen(self.serve() + "/data") as response:
            payload = json.load(response)
            self.assertEqual(response.status, 200)
        self.assertEqual(len(payload["observations"]), 20)
        self.assertEqual(payload["observations"][0]["Symbol"], "50000")

    def test_csv_record_limits_and_strict_parser_do_not_return_partial_data(self):
        self.csv_path.write_bytes(b"x" * 129)
        with patch("dashboard.MAX_CSV_BYTES", 128), self.assertRaisesRegex(ValueError, "byte limit"):
            read_dashboard_data(self.csv_path, stale_after_seconds=30)
        self.write_rows([{"Timestamp_UTC": "2026-01-01T00:00:00Z", "Symbol": "BTC/USDT"}] * 3)
        base_url = self.serve()
        with patch("dashboard.MAX_CSV_ROWS", 2), urlopen(base_url + "/data") as response:
            payload = json.load(response)
        self.assertEqual(len(payload["observations"]), 3)
        self.csv_path.write_text('Timestamp_UTC,Symbol\nnow,"unfinished\n', encoding="utf-8")
        with self.assertRaises(csv.Error):
            read_dashboard_data(self.csv_path, stale_after_seconds=30)

    def test_daily_report_neutralizes_news_formulas_and_bounds_rows(self):
        day = datetime(2026, 10, 25, tzinfo=timezone.utc).date()
        event = {"event_at": "2026-10-25T12:00:00Z", "first_seen_at": "2026-10-25T12:00:00Z",
                 "source": "=BAD()", "payload": {"title": " \t=HYPERLINK(\"https://evil.invalid\")", "source": "+BAD()", "url": "@BAD()", "language": "-BAD()"}}
        store = Mock()
        store.daily_market_summary.return_value = []
        store.source_events.side_effect = [[], [event]]
        with patch("dashboard.get_entsog_flows", return_value={}), patch("dashboard.get_geopolitical_news", return_value={}):
            rows = list(csv.DictReader(io.StringIO(dashboard.daily_report_csv(day, store=store))))
        for column in ("source", "value", "details", "source_url"):
            self.assertTrue(rows[0][column].startswith("'"), (column, rows[0]))
        store.source_events.side_effect = [[], [event, event]]
        with patch("dashboard.MAX_CSV_ROWS", 1), patch("dashboard.get_entsog_flows", return_value={}), patch("dashboard.get_geopolitical_news", return_value={}):
            with self.assertRaisesRegex(ValueError, "row limit"):
                dashboard.daily_report_csv(day, store=store)

    def test_unexpected_errors_and_logs_never_include_exception_details(self):
        self.quiet_http_logs.stop()
        base_url = self.serve()
        marker = "PRIVATE_TOKEN_SECRET"
        with self.assertLogs(level="INFO") as logs:
            with urlopen(base_url + "/api/alerts?token=" + marker):
                pass
            with patch.object(self.history_store, "list_alerts", side_effect=RuntimeError(marker)), self.assertRaises(HTTPError) as error:
                urlopen(base_url + "/api/alerts")
            self.assertEqual(error.exception.code, 500)
            self.assertNotIn(marker, error.exception.read().decode())
            code, response = self.raw_request(base_url, f"GET /?token={marker} HTTP/{marker}\r\n\r\n".encode())
            self.assertEqual(code, 400)
            self.assertNotIn(marker.encode(), response)
        self.assertNotIn(marker, "\n".join(logs.output))
        self.assertNotIn("token=", "\n".join(logs.output))

    def test_missing_metrics_process_is_reset_and_can_recover(self):
        bad, good = Mock(), Mock()
        bad.cpu_percent.return_value = 0.0
        bad.memory_info.side_effect = psutil.NoSuchProcess(123)
        good.memory_info.return_value.rss = 7 * 1024 * 1024
        good.cpu_percent.side_effect = [0.0, 4.5]
        base_url = self.serve()
        with patch.object(dashboard, "SYSTEM_METRICS_PROCESS", None), patch("dashboard.psutil.Process", side_effect=[bad, good]):
            with self.assertRaises(HTTPError) as error:
                urlopen(base_url + "/api/metrics")
            self.assertEqual(error.exception.code, 503)
            with urlopen(base_url + "/api/metrics") as response:
                payload = json.load(response)
        self.assertEqual(payload["system"]["memory_rss_mb"], 7)
        self.assertEqual(payload["system"]["cpu_percent"], 4.5)


if __name__ == "__main__":
    unittest.main()
