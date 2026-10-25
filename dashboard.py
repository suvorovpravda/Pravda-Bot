import argparse
import csv
import json
import logging
import math
import os
from collections import deque
from datetime import datetime, timezone
from functools import partial
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit

from dotenv import load_dotenv


HTML_CONTENT = """<!DOCTYPE html>
<html lang="fr">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Pravda-Bot Control Center</title>
    <style>
        body { font-family: sans-serif; background: #0f172a; color: #f8fafc; margin: 0; padding: 20px; }
        h1 { color: #38bdf8; }
        .card { background: #1e293b; padding: 20px; border-radius: 8px; margin-bottom: 20px; box-shadow: 0 4px 6px rgba(0,0,0,0.3); overflow-x: auto; }
        table { width: 100%; border-collapse: collapse; margin-top: 10px; }
        th, td { padding: 10px; text-align: left; border-bottom: 1px solid #334155; white-space: nowrap; }
        th { background: #334155; }
        .status { font-weight: bold; color: #4ade80; }
    </style>
</head>
<body>
    <h1>Pravda-Bot Control Center</h1>
    <div class="card">
        <h2>État de la télémétrie : <span id="status" class="status">Chargement...</span></h2>
        <p>Seuil minimum configuré : <strong id="threshold">—</strong></p>
        <p>Dernière observation : <span id="last-observation">—</span></p>
    </div>
    <div class="card">
        <h2>Dernières observations</h2>
        <div id="data">Chargement des données...</div>
    </div>
    <script>
        const columns = [
            ["Timestamp_UTC", "UTC"],
            ["Symbol", "Paire"],
            ["Kraken_Price", "Kraken"],
            ["Binance_Price", "Binance"],
            ["Gross_Spread_Percent", "Spread brut (%)"],
            ["Threshold_Reached", "Seuil atteint"],
            ["Market_Data_Status", "Santé des flux"]
        ];

        async function fetchData() {
            const container = document.getElementById("data");
            try {
                const response = await fetch("/data", { cache: "no-store" });
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}`);
                }
                const payload = await response.json();
                document.getElementById("status").textContent = payload.status;
                document.getElementById("threshold").textContent = `${payload.threshold}%`;
                document.getElementById("last-observation").textContent =
                    payload.last_observation || "Aucune";

                const table = document.createElement("table");
                const header = table.createTHead().insertRow();
                for (const [, label] of columns) {
                    const cell = document.createElement("th");
                    cell.textContent = label;
                    header.appendChild(cell);
                }
                const body = table.createTBody();
                for (const row of payload.observations) {
                    const tableRow = body.insertRow();
                    for (const [key] of columns) {
                        const cell = tableRow.insertCell();
                        cell.textContent = row[key] ?? "";
                    }
                }
                if (payload.observations.length === 0) {
                    const row = body.insertRow();
                    const cell = row.insertCell();
                    cell.colSpan = columns.length;
                    cell.textContent = "Aucune observation disponible.";
                }
                container.replaceChildren(table);
            } catch (error) {
                container.textContent = "Erreur de chargement des observations.";
                document.getElementById("status").textContent = "Indisponible";
            }
        }

        fetchData();
        setInterval(fetchData, 5000);
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


class DashboardHandler(BaseHTTPRequestHandler):
    def __init__(
        self,
        *args,
        csv_path=Path("spread_observations.csv"),
        threshold=0.05,
        stale_after_seconds=30.0,
        **kwargs,
    ):
        self.csv_path = Path(csv_path)
        self.threshold = threshold
        self.stale_after_seconds = stale_after_seconds
        super().__init__(*args, **kwargs)

    def do_GET(self):
        request_path = urlsplit(self.path).path
        if request_path == "/":
            body = HTML_CONTENT.encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
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
