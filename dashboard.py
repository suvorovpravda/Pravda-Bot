import argparse
import csv
import json
import logging
import math
import os
import threading
import time
from collections import deque
from datetime import datetime, timezone
from functools import partial
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit

from dotenv import load_dotenv
import psutil

from energy_data import (
    EnergyDataError,
    get_entsog_flows,
    get_geopolitical_news,
    get_ttf_price_status,
)
from live_market_data import get_reference_fx_rates, read_market_snapshot


START_TIME = time.monotonic()
SYSTEM_METRICS_PROCESS = None
SYSTEM_METRICS_PROCESS_LOCK = threading.Lock()


def get_system_metrics():
    global SYSTEM_METRICS_PROCESS

    if SYSTEM_METRICS_PROCESS is None:
        with SYSTEM_METRICS_PROCESS_LOCK:
            if SYSTEM_METRICS_PROCESS is None:
                process = psutil.Process(os.getpid())
                process.cpu_percent(interval=None)
                SYSTEM_METRICS_PROCESS = process

    memory_mb = SYSTEM_METRICS_PROCESS.memory_info().rss / (1024 * 1024)
    cpu_percent = SYSTEM_METRICS_PROCESS.cpu_percent(interval=None)
    return {
        "memory_rss_mb": round(memory_mb, 2),
        "cpu_percent": round(cpu_percent, 1),
        "uptime_seconds": round(time.monotonic() - START_TIME, 1),
    }


HTML_TEMPLATE = Path(__file__).with_name("dashboard.html").read_text(encoding="utf-8")


def read_dashboard_data(csv_path: Path, *, stale_after_seconds: float, now=None):
    observations = deque(maxlen=20)
    if csv_path.exists():
        with csv_path.open("r", encoding="utf-8-sig", newline="") as source:
            observations.extend(csv.DictReader(source))

    ordered_observations = list(reversed(observations))
    last_observation = (
        ordered_observations[0].get("Timestamp_UTC", "") if ordered_observations else ""
    )
    if not last_observation:
        status = "Aucune observation"
    else:
        try:
            timestamp = datetime.fromisoformat(last_observation.replace("Z", "+00:00"))
            if timestamp.tzinfo is None:
                timestamp = timestamp.replace(tzinfo=timezone.utc)
        except ValueError:
            status = "État inconnu"
        else:
            current_time = now or datetime.now(timezone.utc)
            age = (current_time - timestamp).total_seconds()
            if age < -stale_after_seconds:
                status = "Horodatage futur"
            elif age <= stale_after_seconds:
                status = "Données récentes"
            else:
                status = "Données obsolètes"

    return {
        "status": status,
        "last_observation": last_observation,
        "observations": ordered_observations,
    }


def read_lng_dashboard_data(csv_path: Path):
    observations = deque(maxlen=10)
    if csv_path.exists():
        with csv_path.open("r", encoding="utf-8-sig", newline="") as source:
            reader = csv.DictReader(source)
            required_columns = {"Timestamp_UTC", "Benchmark", "Price_USD", "Status", "Note"}
            if reader.fieldnames is None or not required_columns.issubset(reader.fieldnames):
                raise ValueError("LNG CSV has an incompatible schema")
            observations.extend(reader)

    return {
        "status": (
            "Exemples simulés (pas de données de marché réelles)"
            if observations
            else "Aucune observation GNL"
        ),
        "observations": list(reversed(observations)),
    }


class DashboardHandler(BaseHTTPRequestHandler):
    def __init__(
        self,
        *args,
        csv_path=Path("spread_observations.csv"),
        lng_csv_path=Path("lng_observations.csv"),
        market_snapshot_path=Path(
            os.environ.get("MARKET_SNAPSHOT_FILE", "market_snapshot.json")
        ),
        threshold=0.05,
        stale_after_seconds=30.0,
        **kwargs,
    ):
        self.csv_path = Path(csv_path)
        self.lng_csv_path = Path(lng_csv_path)
        self.market_snapshot_path = Path(market_snapshot_path)
        self.threshold = threshold
        self.stale_after_seconds = stale_after_seconds
        super().__init__(*args, **kwargs)

    def _send_json(self, payload, status=200):
        body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Cache-Control", "no-store")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        request_path = urlsplit(self.path).path
        if request_path == "/api/market":
            try:
                payload = read_market_snapshot(
                    self.market_snapshot_path,
                    stale_after_seconds=10,
                )
            except ValueError as error:
                logging.warning("Live market snapshot unavailable (%s)", type(error).__name__)
                self._send_json(
                    {"status": "unavailable", "error": "Market snapshot unavailable"}, 500
                )
                return
            self._send_json(payload)
            return

        if request_path == "/api/fx":
            try:
                self._send_json(get_reference_fx_rates())
            except RuntimeError as error:
                logging.warning("Reference FX rates unavailable (%s)", type(error).__name__)
                self._send_json(
                    {"status": "unavailable", "error": "Reference FX rates unavailable"},
                    502,
                )
            return

        if request_path == "/api/eu/flows":
            try:
                self._send_json(get_entsog_flows())
            except EnergyDataError as error:
                logging.warning("ENTSOG flow data unavailable (%s)", type(error).__name__)
                self._send_json(
                    {"status": "unavailable", "error": "ENTSOG data unavailable"}, 502
                )
            return

        if request_path == "/api/geopolitics":
            try:
                self._send_json(get_geopolitical_news())
            except EnergyDataError as error:
                logging.warning("Geopolitical news unavailable (%s)", type(error).__name__)
                self._send_json(
                    {"status": "unavailable", "error": "News feed unavailable"}, 502
                )
            return

        if request_path == "/api/gas-price":
            self._send_json(get_ttf_price_status())
            return

        if request_path == "/api/metrics":
            try:
                payload = {
                    "status": "success",
                    "system": get_system_metrics(),
                    "timestamp": time.time(),
                }
            except psutil.Error as error:
                logging.warning(
                    "Unable to collect dashboard process metrics (%s)",
                    type(error).__name__,
                )
                payload = {
                    "status": "error",
                    "error": "System metrics unavailable",
                    "timestamp": time.time(),
                }
                body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
                self.send_response(503)
                self.send_header("Content-Type", "application/json; charset=utf-8")
                self.send_header("Cache-Control", "no-store")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
                return
            body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Cache-Control", "no-store")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return

        if request_path == "/":
            body = HTML_TEMPLATE.encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Cache-Control", "no-store")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return

        if request_path == "/lng-data":
            try:
                payload = read_lng_dashboard_data(self.lng_csv_path)
            except (OSError, csv.Error, UnicodeDecodeError, ValueError):
                logging.exception("Unable to read LNG dashboard CSV %s", self.lng_csv_path)
                self.send_error(500, "Unable to read LNG observation log")
                return
            body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Cache-Control", "no-store")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return

        if request_path == "/data":
            try:
                payload = read_dashboard_data(
                    self.csv_path,
                    stale_after_seconds=self.stale_after_seconds,
                )
            except (OSError, csv.Error, UnicodeDecodeError) as error:
                logging.exception("Unable to read dashboard CSV %s", self.csv_path)
                self.send_error(500, "Unable to read observation log")
                return
            payload["threshold"] = self.threshold
            body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Cache-Control", "no-store")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return

        self.send_error(404)


def positive_float(value):
    try:
        parsed = float(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("must be a finite positive number") from error
    if not math.isfinite(parsed) or parsed <= 0:
        raise argparse.ArgumentTypeError("must be a finite positive number")
    return parsed


def non_negative_float(value):
    try:
        parsed = float(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("must be a finite non-negative number") from error
    if not math.isfinite(parsed) or parsed < 0:
        raise argparse.ArgumentTypeError("must be a finite non-negative number")
    return parsed


def main():
    load_dotenv()
    parser = argparse.ArgumentParser(description="Serve the local Pravda-Bot telemetry dashboard.")
    parser.add_argument(
        "--csv",
        type=Path,
        default=Path(os.environ.get("SPREAD_LOG_FILE", "spread_observations.csv")),
        help="Path to spread_observations.csv (default: current directory)",
    )
    parser.add_argument(
        "--lng-csv",
        type=Path,
        default=Path(os.environ.get("LNG_LOG_FILE", "lng_observations.csv")),
        help="Path to lng_observations.csv (default: current directory)",
    )
    parser.add_argument(
        "--port",
        type=int,
        default=os.environ.get("DASHBOARD_PORT", "8080"),
        help="Local HTTP port (default: 8080)",
    )
    parser.add_argument(
        "--threshold",
        type=non_negative_float,
        default=os.environ.get("MIN_SPREAD_THRESHOLD", "0.05"),
        help="Configured gross spread threshold in percent",
    )
    parser.add_argument(
        "--stale-after",
        type=positive_float,
        default=os.environ.get("DASHBOARD_STALE_AFTER_SECONDS", "30"),
        help="Seconds after which the latest observation is considered stale",
    )
    args = parser.parse_args()
    if not 1 <= args.port <= 65535:
        parser.error("--port must be between 1 and 65535")

    handler = partial(
        DashboardHandler,
        csv_path=args.csv,
        lng_csv_path=args.lng_csv,
        threshold=args.threshold,
        stale_after_seconds=args.stale_after,
    )
    server = ThreadingHTTPServer(("127.0.0.1", args.port), handler)
    try:
        print(f"Tableau de bord local : http://127.0.0.1:{args.port}")
        print(f"Journal CSV : {args.csv.resolve()}")
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nArrêt du tableau de bord.")
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
