import argparse
import csv
import json
import logging
import math
import os
import time
from collections import deque
from datetime import datetime, timezone
from functools import partial
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit

from dotenv import load_dotenv
import psutil


START_TIME = time.monotonic()


def get_system_metrics():
    process = psutil.Process(os.getpid())
    memory_mb = process.memory_info().rss / (1024 * 1024)
    cpu_percent = process.cpu_percent(interval=None)
    return {
        "memory_rss_mb": round(memory_mb, 2),
        "cpu_percent": round(cpu_percent, 1),
        "uptime_seconds": round(time.monotonic() - START_TIME, 1),
    }


HTML_TEMPLATE = """<!DOCTYPE html>
<html lang="fr">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Pravda-Bot // Command Center</title>
    <style>
        body { font-family: sans-serif; background: #0f172a; color: #f8fafc; margin: 0; padding: 20px; }
        h1 { color: #38bdf8; }
        .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 440px), 1fr)); gap: 20px; }
        .card { background: #1e293b; padding: 20px; border-radius: 8px; margin-bottom: 20px; box-shadow: 0 4px 6px rgba(0,0,0,0.3); overflow-x: auto; }
        table { width: 100%; border-collapse: collapse; margin-top: 10px; }
        th, td { padding: 10px; text-align: left; border-bottom: 1px solid #334155; white-space: nowrap; }
        th { background: #334155; }
        .status { font-weight: bold; color: #4ade80; }
        .status.error { color: #f87171; }
        .muted { color: #94a3b8; font-size: 0.9rem; }
    </style>
</head>
<body>
    <h1>Pravda-Bot // Command Center</h1>
    <div class="card">
        <h2>État de la télémétrie : <span id="status" class="status">Chargement...</span></h2>
        <p>Seuil minimum configuré : <strong id="threshold">—</strong></p>
        <p>Dernière observation : <span id="last-observation">—</span></p>
    </div>
    <div class="grid">
        <div class="card">
            <h2>Derniers Spreads Crypto</h2>
            <p class="muted">Journal Kraken/Binance · dernière estimation nette et état des flux</p>
            <div id="data">Chargement des données...</div>
        </div>
        <div class="card">
            <h2>Marché GNL &amp; Transport (Simulé)</h2>
            <p class="muted">Exemples simulés uniquement — pas de cotations réelles ni de suivi de navires.</p>
            <div id="lng-status" class="muted">Chargement des observations GNL...</div>
            <div id="lng-data"></div>
        </div>
    </div>
    <script>
        const cryptoColumns = [
            ["Timestamp_UTC", "UTC"],
            ["Symbol", "Paire"],
            ["Estimated_Net_Spread_Percent", "Net estimé (%)"],
            ["Market_Data_Status", "Santé des flux"]
        ];

        function renderTable(container, rows, columns, emptyMessage) {
            const table = document.createElement("table");
            const header = table.createTHead().insertRow();
            for (const [, label] of columns) {
                const cell = document.createElement("th");
                cell.textContent = label;
                header.appendChild(cell);
            }
            const body = table.createTBody();
            for (const row of rows) {
                const tableRow = body.insertRow();
                for (const [key] of columns) {
                    const cell = tableRow.insertCell();
                    cell.textContent = row[key] ?? "";
                }
            }
            if (rows.length === 0) {
                const row = body.insertRow();
                const cell = row.insertCell();
                cell.colSpan = columns.length;
                cell.textContent = emptyMessage;
            }
            container.replaceChildren(table);
        }

        async function fetchCryptoData() {
            const container = document.getElementById("data");
            try {
                const response = await fetch("/data", { cache: "no-store" });
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}`);
                }
                const payload = await response.json();
                const status = document.getElementById("status");
                status.textContent = payload.status;
                status.classList.remove("error");
                document.getElementById("threshold").textContent = `${payload.threshold}%`;
                document.getElementById("last-observation").textContent =
                    payload.last_observation || "Aucune";
                renderTable(container, payload.observations, cryptoColumns,
                    "Aucune observation crypto disponible.");
            } catch (error) {
                container.textContent = "Erreur de chargement des observations.";
                const status = document.getElementById("status");
                status.textContent = "Indisponible";
                status.classList.add("error");
            }
        }

        const lngColumns = [
            ["Timestamp_UTC", "UTC"],
            ["Benchmark", "Benchmark"],
            ["Price_USD", "Prix exemple (USD)"],
            ["Status", "Source"],
            ["Note", "Note"]
        ];

        async function fetchLngData() {
            const container = document.getElementById("lng-data");
            const status = document.getElementById("lng-status");
            try {
                const response = await fetch("/lng-data", { cache: "no-store" });
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}`);
                }
                const payload = await response.json();
                status.textContent = payload.status;
                renderTable(container, payload.observations, lngColumns,
                    "Aucune observation GNL. Lancez lng_observations.py pour créer des exemples.");
            } catch (error) {
                status.textContent = "Journal GNL indisponible.";
                container.textContent = "Impossible de lire les observations GNL.";
            }
        }

        fetchCryptoData();
        fetchLngData();
        setInterval(fetchCryptoData, 5000);
        setInterval(fetchLngData, 5000);
    </script>
</body>
</html>
"""


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
        threshold=0.05,
        stale_after_seconds=30.0,
        **kwargs,
    ):
        self.csv_path = Path(csv_path)
        self.lng_csv_path = Path(lng_csv_path)
        self.threshold = threshold
        self.stale_after_seconds = stale_after_seconds
        super().__init__(*args, **kwargs)

    def do_GET(self):
        request_path = urlsplit(self.path).path
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
