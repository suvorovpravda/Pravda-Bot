import argparse
import base64
import csv
import hashlib
import http.client
import io
import json
import logging
import math
import os
import re
import sqlite3
import stat
import threading
import time
from collections import OrderedDict, deque
from datetime import datetime, timezone
from functools import partial
from importlib.resources import files
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
from ais_proxy import get_ais_recent
from security_and_fees import csv_safe_text


START_TIME = time.monotonic()
SYSTEM_METRICS_PROCESS = None
SYSTEM_METRICS_PROCESS_LOCK = threading.Lock()
RATE_LIMITER_CREATION_LOCK = threading.Lock()
DEFAULT_RATE_LIMIT_PER_MINUTE = 240  # The 1 Hz UI plus logs/alerts uses over 85 requests/minute.
MAX_HEADER_COUNT = 64
MAX_HEADER_BYTES = 16 * 1024
MAX_HEADER_LINE_BYTES = 8 * 1024
MAX_REQUEST_TARGET_BYTES = 2048
MAX_REQUEST_BODY_BYTES = 4096
MAX_CSV_BYTES = 8 * 1024 * 1024
MAX_CSV_ROWS = 50000  # Daily report export cap; observation reads use a bounded tail.
CSV_TAIL_CHUNK_BYTES = 64 * 1024
MAX_CSV_HEADER_BYTES = 16 * 1024


def rate_limit_setting(value):
    try:
        if isinstance(value, bool) or not isinstance(value, (str, int)):
            raise ValueError
        limit = int(value)
    except (TypeError, ValueError, OverflowError) as error:
        raise argparse.ArgumentTypeError("must be an integer between 1 and 10000") from error
    if not 1 <= limit <= 10000:
        raise argparse.ArgumentTypeError("must be an integer between 1 and 10000")
    return limit


class RequestRateLimiter:
    def __init__(self, limit, *, max_clients=128):
        self.limit = rate_limit_setting(limit)
        if isinstance(max_clients, bool) or not isinstance(max_clients, int) or not 1 <= max_clients <= 128:
            raise ValueError("max_clients must be between 1 and 128")
        self.max_clients = max_clients
        self._clients = OrderedDict()
        self._lock = threading.Lock()

    def retry_after(self, client, *, now=None):
        current = time.monotonic() if now is None else now
        with self._lock:
            requests = self._clients.pop(client, deque())
            while requests and requests[0] <= current - 60:
                requests.popleft()
            while len(self._clients) >= self.max_clients:
                self._clients.popitem(last=False)
            self._clients[client] = requests
            if len(requests) >= self.limit:
                return max(1, math.ceil(requests[0] + 60 - current))
            requests.append(current)
            return 0


class _LimitedHeaderReader:
    def __init__(self, source):
        self.source = source
        self.count = 0
        self.total = 0

    def readline(self, limit):
        line = self.source.readline(min(limit, MAX_HEADER_LINE_BYTES + 1))
        if len(line) > MAX_HEADER_LINE_BYTES:
            raise http.client.LineTooLong("header line")
        self.total += len(line)
        if line not in (b"\r\n", b"\n", b""):
            self.count += 1
        if self.count > MAX_HEADER_COUNT or self.total > MAX_HEADER_BYTES:
            raise http.client.HTTPException("request headers exceed dashboard limits")
        return line


def _read_csv_tail(source, *, start, end, keep, budget):
    """Read recent complete RFC-style CSV records backwards within a byte budget.

    A quote changes state in either direction; doubled quotes change it twice.
    This preserves embedded newlines produced by csv.writer. Only complete
    records are parsed when the beginning of the byte window cuts a record.
    """
    parts = []
    position = end
    loaded = 0
    quoted = False
    has_content = False
    records = 0
    selected_start = None
    while position > start and loaded < budget:
        width = min(CSV_TAIL_CHUNK_BYTES, position - start, budget - loaded)
        position -= width
        source.seek(position)
        block = source.read(width)
        if len(block) != width:
            raise ValueError("Observation log changed during reading")
        parts.append(block)
        loaded += width
        for index in range(len(block) - 1, -1, -1):
            value = block[index]
            if value == 34:
                quoted = not quoted
            if value == 10 and not quoted:
                if has_content:
                    records += 1
                    selected_start = position + index + 1
                    if records >= keep:
                        return b"".join(reversed(parts))[selected_start - position:]
                has_content = False
            elif value not in (10, 13):
                has_content = True
    if position == start:
        if quoted:
            raise csv.Error("unexpected end of data in quoted observation")
        return b"".join(reversed(parts))
    if selected_start is None:
        raise ValueError("Observation record exceeds the byte limit")
    return b"".join(reversed(parts))[selected_start - position:]


def _read_observation_rows(csv_path, *, keep, required_columns):
    observations = deque(maxlen=keep)
    try:
        descriptor = os.open(csv_path, os.O_RDONLY | getattr(os, "O_NONBLOCK", 0))
    except FileNotFoundError:
        return observations
    with os.fdopen(descriptor, "rb") as source:
        metadata = os.fstat(source.fileno())
        if not stat.S_ISREG(metadata.st_mode):
            raise ValueError("Observation log must be a regular file")
        if not metadata.st_size:
            return observations
        header_limit = min(MAX_CSV_HEADER_BYTES, MAX_CSV_BYTES)
        header = source.readline(header_limit + 1)
        if len(header) > header_limit:
            raise ValueError("Observation header exceeds the byte limit")
        fields = next(csv.reader(io.StringIO(header.decode("utf-8-sig"), newline=""), strict=True), None)
        if (fields is None or len(set(fields)) != len(fields)
                or not required_columns.issubset(fields)):
            raise ValueError("Observation log has an incompatible schema")
        if metadata.st_size == len(header):
            return observations
        content = _read_csv_tail(
            source, start=len(header), end=metadata.st_size, keep=keep,
            budget=MAX_CSV_BYTES - len(header),
        )
    reader = csv.DictReader(io.StringIO(content.decode("utf-8"), newline=""), fieldnames=fields, strict=True)
    for row in reader:
        if None in row or any(row.get(column) is None for column in required_columns):
            raise ValueError("Observation log contains an incomplete row")
        observations.append(row)
    return observations


def _csv_safe_cell(value):
    return csv_safe_text(value) if isinstance(value, str) else value


def get_system_metrics():
    global SYSTEM_METRICS_PROCESS

    with SYSTEM_METRICS_PROCESS_LOCK:
        if SYSTEM_METRICS_PROCESS is None:
            process = psutil.Process(os.getpid())
            process.cpu_percent(interval=None)
            SYSTEM_METRICS_PROCESS = process
        try:
            memory_mb = SYSTEM_METRICS_PROCESS.memory_info().rss / (1024 * 1024)
            cpu_percent = SYSTEM_METRICS_PROCESS.cpu_percent(interval=None)
        except psutil.NoSuchProcess:
            SYSTEM_METRICS_PROCESS = None
            raise
    return {
        "memory_rss_mb": round(memory_mb, 2),
        "cpu_percent": round(cpu_percent, 1),
        "uptime_seconds": round(time.monotonic() - START_TIME, 1),
    }


_template_path = Path(__file__).with_name("dashboard.html")
HTML_TEMPLATE = (
    _template_path.read_text(encoding="utf-8") if _template_path.is_file()
    else files("pravda_resources").joinpath("dashboard.html").read_text(encoding="utf-8")
)
INLINE_SCRIPT_HASHES = " ".join(
    "'sha256-" + base64.b64encode(hashlib.sha256(script.encode("utf-8")).digest()).decode("ascii") + "'"
    for script in re.findall(r"<script(?:\s[^>]*)?>(.*?)</script\s*>", HTML_TEMPLATE, re.I | re.S)
)


def read_dashboard_data(csv_path: Path, *, stale_after_seconds: float, now=None):
    observations = _read_observation_rows(csv_path, keep=20, required_columns={"Timestamp_UTC", "Symbol"})

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
    observations = _read_observation_rows(
        csv_path, keep=10, required_columns={"Timestamp_UTC", "Benchmark", "Price_USD", "Status", "Note"}
    )

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
    row_count = 0

    def write_row(row):
        nonlocal row_count
        row_count += 1
        if row_count > MAX_CSV_ROWS:
            raise ValueError("Daily report exceeds the row limit")
        writer.writerow({key: _csv_safe_cell(value) for key, value in row.items()})
        if output.tell() > MAX_CSV_BYTES:
            raise ValueError("Daily report exceeds the size limit")

    def write_source_error(source, error):
        write_row(
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
        write_row(
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
        write_row(
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
        write_row(
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
    default_request_version = "HTTP/1.0"
    connection_timeout_seconds = 5.0

    def log_message(self, format, *args):
        logging.info("Dashboard HTTP request handled")

    def log_error(self, format, *args):
        logging.warning("Dashboard HTTP request interrupted")

    def setup(self):
        self.request.settimeout(self.connection_timeout_seconds)
        super().setup()

    def version_string(self):
        return "PravdaDashboard"

    def end_headers(self):
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("X-Frame-Options", "DENY")
        self.send_header("Referrer-Policy", "no-referrer")
        self.send_header("Permissions-Policy", "camera=(), microphone=(), geolocation=(), payment=(), usb=()")
        self.send_header("Content-Security-Policy", f"default-src 'self'; script-src 'self' {INLINE_SCRIPT_HASHES}; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-src https://open.spotify.com; frame-ancestors 'none'; base-uri 'none'; form-action 'self'")
        super().end_headers()

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
        rate_limit_per_minute=None,
        **kwargs,
    ):
        self.csv_path = Path(csv_path)
        self.lng_csv_path = Path(lng_csv_path)
        self.market_snapshot_path = Path(market_snapshot_path)
        self.market_history_store = market_history_store or get_market_store()
        self.threshold = threshold
        self.stale_after_seconds = stale_after_seconds
        self.rate_limit_per_minute = rate_limit_setting(
            rate_limit_per_minute if rate_limit_per_minute is not None else
            os.environ.get("DASHBOARD_RATE_LIMIT_PER_MINUTE", str(DEFAULT_RATE_LIMIT_PER_MINUTE))
        )
        self._response_started = False
        super().__init__(*args, **kwargs)

    def send_response(self, code, message=None):
        self._response_started = True
        super().send_response(code, message)

    def send_error(self, code, message=None, explain=None):
        self.close_connection = True
        self._send_json({"status": "unavailable" if code >= 500 else "error",
                         "error": self.responses.get(code, ("Request failed",))[0]}, code)

    def _send_json(self, payload, status=200, *, headers=None):
        body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Cache-Control", "no-store")
        self.send_header("Content-Length", str(len(body)))
        for name, value in (headers or {}).items():
            self.send_header(name, value)
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(body)

    def _send_csv(self, payload, filename):
        body = payload.encode("utf-8")
        if len(body) > MAX_CSV_BYTES:
            self._send_json({"status": "unavailable", "error": "CSV export exceeds the size limit"}, 503)
            return
        filename = re.sub(r"[^A-Za-z0-9._-]", "_", filename)
        self.send_response(200)
        self.send_header("Content-Type", "text/csv; charset=utf-8")
        self.send_header("Content-Disposition", f'attachment; filename="{filename}"')
        self.send_header("Cache-Control", "no-store")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _same_origin_request(self):
        try:
            hosts = self.headers.get_all("Host", [])
            origins = self.headers.get_all("Origin", [])
            if len(hosts) != 1 or len(origins) != 1:
                return False
            authority = urlsplit("//" + hosts[0])
            origin = urlsplit(origins[0])
            return (
                origin.scheme == "http" and origin.hostname == authority.hostname
                and (origin.port if origin.port is not None else 80) == self.server.server_port
                and origin.username is None and origin.password is None
                and not (origin.path or origin.query or origin.fragment)
                and not any(character.isspace() for character in origins[0])
            )
        except ValueError:
            return False

    def parse_request(self):
        original = self.rfile
        self.rfile = _LimitedHeaderReader(original)
        try:
            if not super().parse_request():
                return False
        finally:
            self.rfile = original
        if len(self.path.encode("iso-8859-1")) > MAX_REQUEST_TARGET_BYTES:
            self.send_error(414)
            return False
        try:
            hosts = self.headers.get_all("Host", [])
            if len(hosts) != 1 or not hosts[0] or any(character.isspace() for character in hosts[0]):
                raise ValueError
            authority = urlsplit("//" + hosts[0])
            if (authority.hostname not in {"localhost", "127.0.0.1", "::1"}
                    or authority.username is not None or authority.password is not None
                    or authority.path or authority.query or authority.fragment
                    or (authority.port if authority.port is not None else 80) != self.server.server_port):
                raise ValueError
            origins = self.headers.get_all("Origin", [])
            if len(origins) > 1 or (origins and not self._same_origin_request()):
                raise ValueError
            if self.command in {"POST", "DELETE"} and not self._same_origin_request():
                raise ValueError
            fetch_sites = self.headers.get_all("Sec-Fetch-Site", [])
            if len(fetch_sites) > 1 or (fetch_sites and fetch_sites[0] not in {"same-origin", "none"}):
                raise ValueError
            # This legacy GET changes each alert's latch, so it needs browser-origin proof too.
            if (self.command == "GET" and self.path.partition("?")[0] == "/api/alerts/check"
                    and not self._same_origin_request() and fetch_sites != ["same-origin"]):
                raise ValueError
        except ValueError:
            self._send_json({"status": "forbidden", "error": "Local origin required"}, 403)
            self.close_connection = True
            return False
        lengths = self.headers.get_all("Content-Length", [])
        if len(lengths) > 1 or self.headers.get_all("Transfer-Encoding", []):
            self.send_error(400)
            return False
        if lengths:
            length = lengths[0]
            if not length.isascii() or not length.isdigit() or len(length) > 10:
                self.send_error(400)
                return False
            allowed = MAX_REQUEST_BODY_BYTES if self.command == "POST" else 0
            if int(length) > allowed:
                self.send_error(413)
                return False
        with RATE_LIMITER_CREATION_LOCK:
            if not hasattr(self.server, "dashboard_rate_limiter"):
                self.server.dashboard_rate_limiter = RequestRateLimiter(self.rate_limit_per_minute)
            limiter = self.server.dashboard_rate_limiter
        retry_after = limiter.retry_after(self.client_address[0])
        if retry_after:
            self._send_json({"status": "unavailable", "error": "Too many requests"}, 429,
                            headers={"Retry-After": str(retry_after)})
            self.close_connection = True
            return False
        try:
            target = urlsplit(self.path)
            if target.scheme or target.netloc or target.fragment or not self.path.startswith("/"):
                raise ValueError
            parse_qs(target.query, max_num_fields=32)
        except ValueError:
            self.send_error(400)
            return False
        return True

    def _run_route(self, callback):
        try:
            callback()
        except TimeoutError:
            self.close_connection = True
            if not self._response_started:
                self.send_error(408)
        except (BrokenPipeError, ConnectionResetError):
            self.close_connection = True
        except Exception as error:
            logging.warning("Dashboard request unavailable (%s)", type(error).__name__)
            self.close_connection = True
            if not self._response_started:
                self._send_json({"status": "unavailable", "error": "Dashboard data unavailable"}, 500)

    def do_GET(self):
        self._run_route(self._serve_GET)

    def _serve_GET(self):
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
                self._send_json({"status": "error", "error": "Invalid history query"}, 400)
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
                self._send_json({"status": "error", "error": "Invalid history query"}, 400)
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
            except (OSError, sqlite3.Error, ValueError) as error:
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
                logging.warning("Unable to persist ENTSOG observations (%s)", type(error).__name__)
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
            except (OSError, sqlite3.Error, ValueError) as error:
                logging.warning("Unable to persist news provenance (%s)", type(error).__name__)
                self._send_json(
                    {"status": "error", "error": "Unable to persist news provenance"}, 500
                )
            return

        if request_path == "/api/gas-price":
            self._send_json(get_ttf_price_status())
            return

        if request_path in {"/api/ais/recent", "/api/ais/geojson"}:
            try:
                limit = int(query.get("limit", [100])[0])
                if not 1 <= limit <= 2000:
                    raise ValueError
            except (ValueError, TypeError):
                self._send_json({"status": "error", "error": "limit must be between 1 and 2000"}, 400)
                return
            messages = get_ais_recent(limit)
            if not messages:
                self._send_json({"status": "unavailable", "error": "No verified AIS provider connected"}, 503)
                return
            rows = [{"mmsi": m.mmsi, "lat": m.lat, "lon": m.lon, "sog": m.sog,
                     "cog": m.cog, "timestamp": m.timestamp, "source": m.source} for m in messages]
            if request_path.endswith("geojson"):
                self._send_json({"type": "FeatureCollection", "features": [
                    {"type": "Feature", "geometry": {"type": "Point", "coordinates": [m["lon"], m["lat"]]},
                     "properties": m} for m in rows]})
            else:
                self._send_json({"status": "available", "count": len(rows), "messages": rows})
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
                logging.warning("LNG observation log unavailable")
                self._send_json({"status": "unavailable", "error": "LNG observation log unavailable", "observations": []}, 500)
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
            except (OSError, csv.Error, UnicodeDecodeError, ValueError):
                logging.warning("Observation log unavailable")
                self._send_json({"status": "unavailable", "error": "Observation log unavailable", "observations": []}, 500)
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
        self._run_route(self._serve_POST)

    def _serve_POST(self):
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
            content = self.rfile.read(content_length)
            if len(content) != content_length:
                raise ValueError("incomplete body")
            payload = json.loads(content)
            if not isinstance(payload, dict):
                raise ValueError("alert payload must be a JSON object")
            alert = self.market_history_store.create_alert(
                payload.get("symbol"),
                payload.get("exchange", "Binance"),
                payload.get("currency"),
                payload.get("direction"),
                payload.get("target"),
            )
        except (ValueError, UnicodeDecodeError, json.JSONDecodeError):
            self._send_json({"status": "error", "error": "Invalid alert payload"}, 400)
            return
        except (OSError, sqlite3.Error) as error:
            logging.warning("Unable to persist price alert (%s)", type(error).__name__)
            self._send_json({"status": "error", "error": "Unable to save price alert"}, 500)
            return
        self._send_json({"status": "created", "alert": alert}, 201)

    def do_DELETE(self):
        self._run_route(self._serve_DELETE)

    def _serve_DELETE(self):
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
    parser.add_argument(
        "--rate-limit", type=rate_limit_setting,
        default=os.environ.get("DASHBOARD_RATE_LIMIT_PER_MINUTE", str(DEFAULT_RATE_LIMIT_PER_MINUTE)),
        help="Requests per minute per client (default: 240, for the dashboard's 1 Hz polling)",
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
        rate_limit_per_minute=args.rate_limit,
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
