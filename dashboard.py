import argparse
import csv
import io
import json
import logging
import math
import os
import sqlite3
import threading
import time
from collections import deque
from datetime import datetime, timezone
from functools import partial
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlsplit

from dotenv import load_dotenv
import psutil

from energy_data import (
    EnergyDataError,
    get_entsog_flows,
    get_geopolitical_news,
    get_ttf_price_status,
)
from live_market_data import get_reference_fx_rates, read_market_snapshot
from market_history import get_market_store
try:
    from ais_proxy import get_ais_recent
    AIS_AVAILABLE = True
except Exception:
    AIS_AVAILABLE = False
    get_ais_recent = None


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


def daily_report_csv(report_date, *, store=None):
    store = store or get_market_store()
    columns = (
        "record_type",
        "date_utc",
        "source",
        "status",
        "symbol",
        "exchange",
        "metric",
        "value",
        "unit",
        "details",
        "source_url",
    )
    output = io.StringIO(newline="")
    writer = csv.DictWriter(output, fieldnames=columns)
    writer.writeheader()

    def write_source_error(source, error):
        writer.writerow(
            {
                "record_type": "source_status",
                "date_utc": report_date.isoformat(),
                "source": source,
                "status": "unavailable",
                "details": f"Source fetch failed ({type(error).__name__})",
            }
        )

    for summary in store.daily_market_summary(report_date):
        opening, closing = summary["open"], summary["close"]
        change = (closing - opening) / opening * 100 if opening else None
        writer.writerow(
            {
                "record_type": "market_summary",
                "date_utc": report_date.isoformat(),
                "source": "Kraken public ticker" if summary["exchange"] == "Kraken" else "Binance public ticker",
                "status": "observed",
                "symbol": summary["symbol"],
                "exchange": summary["exchange"],
                "metric": "open;high;low;close;change_percent;tick_count",
                "value": ";".join(
                    "" if value is None else str(value)
                    for value in (
                        opening,
                        summary["high"],
                        summary["low"],
                        closing,
                        change,
                        summary["ticks"],
                    )
                ),
                "unit": "USDT",
                "details": "Observed ticks saved locally; daily extrema use available observations only.",
            }
        )

    flows = {}
    try:
        flows = get_entsog_flows()
        store.record_source_events(
            "energy_flow_observation",
            flows.get("source", "ENTSOG"),
            flows.get("observations", []),
        )
    except EnergyDataError as error:
        write_source_error("ENTSOG Transparency Platform", error)
    for event in store.source_events("energy_flow_observation", report_date):
        flow = event["payload"]
        source_timestamp = event["event_at"] or event["first_seen_at"]
        writer.writerow(
            {
                "record_type": "energy_flow_observation",
                "date_utc": source_timestamp,
                "source": event["source"],
                "status": "observed",
                "metric": f"{flow.get('direction', '')} · {flow.get('point', '')}",
                "value": flow.get("value"),
                "unit": flow.get("unit", ""),
                "details": (
                    f"{flow.get('country', '')} · {flow.get('operator', '')}"
                    if event["event_at"]
                    else f"Source timestamp missing; first observed {event['first_seen_at']}"
                ),
                "source_url": flows.get("source_url", "https://transparency.entsog.eu/"),
            }
        )

    news = {}
    try:
        news = get_geopolitical_news()
        store.record_source_events(
            "reported_news",
            news.get("source", "GDELT / BBC World"),
            news.get("articles", []),
        )
    except EnergyDataError as error:
        write_source_error("GDELT / BBC World", error)
    for event in store.source_events("reported_news", report_date):
        article = event["payload"]
        writer.writerow(
            {
                "record_type": "reported_news",
                "date_utc": event["event_at"] or event["first_seen_at"],
                "source": article.get("source") or event["source"],
                "status": "reported_unverified",
                "metric": "news_item",
                "value": article.get("title", ""),
                "details": (
                    article.get("language", "")
                    if event["event_at"]
                    else f"Publication timestamp missing; first observed {event['first_seen_at']}"
                ),
                "source_url": article.get("url", ""),
            }
        )

    return output.getvalue()


def source_status_report(*, market_snapshot_path):
    sources = []
    try:
        market = read_market_snapshot(market_snapshot_path)
    except (OSError, ValueError) as error:
        market = {"status": "unavailable", "symbols": {}}
        market_error = type(error).__name__
    else:
        market_error = ""
    for exchange in ("Kraken", "Binance"):
        exchange_quotes = [
            quote for symbol in market.get("symbols", {}).values()
            if isinstance(symbol, dict)
            for name, quote in symbol.items()
            if name == exchange
        ]
        latest = max(exchange_quotes, key=lambda quote: quote.get("received_at", ""), default=None)
        sources.append(
            {
                "name": f"{exchange} public ticker",
                "status": (
                    "live" if any(quote.get("status") == "live" for quote in exchange_quotes)
                    else "stale" if latest else "unavailable"
                ),
                "last_event_at": latest.get("received_at") if latest else None,
                "age_seconds": latest.get("age_seconds") if latest else None,
                "detail": market_error,
            }
        )
    try:
        fx = get_reference_fx_rates()
        sources.append(
            {
                "name": fx.get("source", "ECB reference FX"),
                "status": fx.get("freshness", "delayed"),
                "last_event_at": fx.get("date"),
                "age_seconds": fx.get("source_age_seconds"),
                "detail": "Daily reference rates; not live FX.",
            }
        )
    except RuntimeError as error:
        sources.append({"name": "ECB reference FX", "status": "unavailable", "detail": type(error).__name__})
    for name, getter in (
        ("ENTSOG Transparency Platform", get_entsog_flows),
        ("GDELT / BBC World", get_geopolitical_news),
    ):
        try:
            payload = getter()
            sources.append(
                {
                    "name": payload.get("source", name),
                    "status": (
                        "delayed"
                        if payload.get("status") == "available"
                        else payload.get("status", "unavailable")
                    ),
                    "last_event_at": payload.get("latest_source_update")
                    or payload.get("retrieved_at"),
                    "detail": payload.get("notice", ""),
                }
            )
        except EnergyDataError as error:
            sources.append({"name": name, "status": "unavailable", "detail": type(error).__name__})
    ttf = get_ttf_price_status()
    sources.append(
        {
            "name": "TTF market data",
            "status": ttf.get("status", "unavailable"),
            "last_event_at": ttf.get("as_of"),
            "detail": ttf.get("notice", ""),
        }
    )
    return {"status": "available", "retrieved_at": datetime.now(timezone.utc).isoformat(), "sources": sources}


class DashboardHandler(BaseHTTPRequestHandler):
    def __init__(
        self,
        *args,
        csv_path=Path("spread_observations.csv"),
        lng_csv_path=Path("lng_observations.csv"),
        market_snapshot_path=Path(
            os.environ.get("MARKET_SNAPSHOT_FILE", "market_snapshot.json")
        ),
        market_history_store=None,
        threshold=0.05,
        stale_after_seconds=30.0,
        **kwargs,
    ):
        self.csv_path = Path(csv_path)
        self.lng_csv_path = Path(lng_csv_path)
        self.market_snapshot_path = Path(market_snapshot_path)
        self.market_history_store = market_history_store or get_market_store()
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

    def _send_csv(self, payload, filename):
        body = payload.encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "text/csv; charset=utf-8")
        self.send_header("Content-Disposition", f'attachment; filename="{filename}"')
        self.send_header("Cache-Control", "no-store")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _same_origin_request(self):
        origin = self.headers.get("Origin")
        host = self.headers.get("Host")
        if not origin:
            return True
        try:
            parsed = urlsplit(origin)
        except ValueError:
            return False
        return parsed.scheme in {"http", "https"} and parsed.netloc == host

    def do_GET(self):
        parsed_request = urlsplit(self.path)
        request_path = parsed_request.path
        query = parse_qs(parsed_request.query)
        if request_path == "/api/market/history":
            symbol = query.get("symbol", ["BTC/USDT"])[0]
            exchange = query.get("exchange", ["Binance"])[0]
            period = query.get("period", ["day"])[0]
            try:
                points = self.market_history_store.history(symbol, exchange, period)
            except ValueError as error:
                self._send_json({"status": "error", "error": str(error)}, 400)
                return
            self._send_json(
                {
                    "status": "available" if points else "waiting",
                    "symbol": symbol,
                    "exchange": exchange,
                    "period": period,
                    "source": "Locally persisted public exchange ticker observations",
                    "observations": points,
                }
            )
            return

        if request_path == "/api/market/history.csv":
            symbol = query.get("symbol", ["BTC/USDT"])[0]
            exchange = query.get("exchange", ["Binance"])[0]
            period = query.get("period", ["day"])[0]
            try:
                payload = self.market_history_store.export_history(symbol, exchange, period)
            except ValueError as error:
                self._send_json({"status": "error", "error": str(error)}, 400)
                return
            filename = f"market-history-{symbol.replace('/', '-')}-{exchange.lower()}-{period}.csv"
            self._send_csv(payload, filename)
            return

        if request_path == "/api/daily-report.csv":
            try:
                report_date = datetime.strptime(
                    query.get("date", [datetime.now(timezone.utc).date().isoformat()])[0],
                    "%Y-%m-%d",
                ).date()
            except ValueError:
                self._send_json({"status": "error", "error": "date must use YYYY-MM-DD"}, 400)
                return
            try:
                payload = daily_report_csv(report_date, store=self.market_history_store)
            except (OSError, sqlite3.Error) as error:
                logging.warning("Daily report unavailable (%s)", type(error).__name__)
                self._send_json({"status": "error", "error": "Daily report unavailable"}, 500)
                return
            self._send_csv(payload, f"pravda-daily-report-{report_date.isoformat()}.csv")
            return

        if request_path == "/api/alerts":
            self._send_json(
                {"status": "available", "alerts": self.market_history_store.list_alerts()}
            )
            return

        if request_path == "/api/alerts/check":
            try:
                market = read_market_snapshot(self.market_snapshot_path, stale_after_seconds=10)
                fx = get_reference_fx_rates()
            except (OSError, ValueError, RuntimeError) as error:
                logging.warning("Price alerts cannot be checked (%s)", type(error).__name__)
                self._send_json(
                    {"status": "unavailable", "error": "Market or FX source unavailable"}, 503
                )
                return
            self._send_json(
                {
                    "status": "available",
                    "triggered": self.market_history_store.check_alerts(market, fx),
                }
            )
            return

        if request_path == "/api/sources":
            self._send_json(source_status_report(market_snapshot_path=self.market_snapshot_path))
            return

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
                payload = get_entsog_flows()
                self.market_history_store.record_source_events(
                    "energy_flow_observation",
                    payload.get("source", "ENTSOG"),
                    payload.get("observations", []),
                )
                self._send_json(payload)
            except EnergyDataError as error:
                logging.warning("ENTSOG flow data unavailable (%s)", type(error).__name__)
                self._send_json(
                    {"status": "unavailable", "error": "ENTSOG data unavailable"}, 502
                )
            except (OSError, sqlite3.Error, ValueError) as error:
                logging.exception("Unable to persist ENTSOG observations")
                self._send_json(
                    {"status": "error", "error": "Unable to persist energy observations"}, 500
                )
            return

        if request_path == "/api/geopolitics":
            try:
                payload = get_geopolitical_news()
                self.market_history_store.record_source_events(
                    "reported_news",
                    payload.get("source", "GDELT / BBC World"),
                    payload.get("articles", []),
                )
                self._send_json(payload)
            except EnergyDataError as error:
                logging.warning("Geopolitical news unavailable (%s)", type(error).__name__)
                self._send_json(
                    {"status": "unavailable", "error": "News feed unavailable"}, 502
                )
            except (OSError, sqlite3.Error, ValueError):
                logging.exception("Unable to persist news provenance")
                self._send_json(
                    {"status": "error", "error": "Unable to persist news provenance"}, 500
                )
            return

        if request_path == "/api/gas-price":
            self._send_json(get_ttf_price_status())
            return

        if request_path == "/api/ais/recent":
            if not AIS_AVAILABLE:
                self._send_json({"status":"unavailable","error":"AIS proxy not configured"},503)
                return
            limit = int(query.get("limit",[100])[0] or 100)
            msgs = get_ais_recent(limit)
            self._send_json({
                "status":"available",
                "source":"aisstream.io",
                "usage":"non-commercial proxy",
                "badge":"non-commercial proxy",
                "note":"Couverture terrestre uniquement, pas de données satellite, quota API. Données non vérifiées.",
                "count":len(msgs),
                "messages":[{
                    "mmsi":m.mmsi,
                    "lat":m.lat,
                    "lon":m.lon,
                    "sog":m.sog,
                    "cog":m.cog,
                    "timestamp":m.timestamp,
                    "source":m.source
                } for m in msgs]
            })
            return

        if request_path == "/api/ais/geojson":
            if not AIS_AVAILABLE:
                self._send_json({"status":"unavailable","error":"AIS proxy not configured"},503)
                return
            limit = int(query.get("limit",[200])[0] or 200)
            msgs = get_ais_recent(limit)
            features = []
            for m in msgs:
                features.append({
                    "type":"Feature",
                    "geometry":{"type":"Point","coordinates":[m.lon,m.lat]},
                    "properties":{
                        "mmsi":m.mmsi,
                        "sog":m.sog,
                        "cog":m.cog,
                        "timestamp":m.timestamp,
                        "source":m.source,
                        "badge":"non-commercial proxy"
                    }
                })
            geojson = {"type":"FeatureCollection","features":features}
            self._send_json(geojson)
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

    def do_POST(self):
        if urlsplit(self.path).path != "/api/alerts":
            self.send_error(404)
            return
        if not self._same_origin_request():
            self._send_json({"status": "error", "error": "Cross-origin requests are not allowed"}, 403)
            return
        if self.headers.get_content_type() != "application/json":
            self._send_json({"status": "error", "error": "Content-Type must be application/json"}, 415)
            return
        try:
            content_length = int(self.headers.get("Content-Length", "0"))
            if not 0 < content_length <= 4096:
                raise ValueError("request body size must be between 1 and 4096 bytes")
            payload = json.loads(self.rfile.read(content_length))
            if not isinstance(payload, dict):
                raise ValueError("alert payload must be a JSON object")
            alert = self.market_history_store.create_alert(
                payload.get("symbol"),
                payload.get("exchange", "Binance"),
                payload.get("currency"),
                payload.get("direction"),
                payload.get("target"),
            )
        except (ValueError, json.JSONDecodeError) as error:
            self._send_json({"status": "error", "error": str(error)}, 400)
            return
        except (OSError, sqlite3.Error) as error:
            logging.exception("Unable to persist price alert")
            self._send_json({"status": "error", "error": "Unable to save price alert"}, 500)
            return
        self._send_json({"status": "created", "alert": alert}, 201)

    def do_DELETE(self):
        if not self._same_origin_request():
            self._send_json({"status": "error", "error": "Cross-origin requests are not allowed"}, 403)
            return
        path = urlsplit(self.path).path
        prefix = "/api/alerts/"
        if not path.startswith(prefix):
            self.send_error(404)
            return
        try:
            alert_id = int(path[len(prefix):])
            if alert_id <= 0:
                raise ValueError
        except ValueError:
            self._send_json({"status": "error", "error": "alert id must be positive"}, 400)
            return
        if not self.market_history_store.delete_alert(alert_id):
            self._send_json({"status": "error", "error": "alert not found"}, 404)
            return
        self._send_json({"status": "deleted", "id": alert_id})

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
