import asyncio
import csv
import json
import math
import os
import sqlite3
import threading
import time
from contextlib import asynccontextmanager
from dataclasses import dataclass
from urllib.parse import urlsplit
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime

import ccxt
import requests
import websockets
from dotenv import load_dotenv
from rich.console import Console
from rich.markup import escape
from secure_http import request as secure_request, SecureHTTPError

load_dotenv()

from alert_gate import AlertGate
from bot_config import load_config
from bot_dashboard import SpreadSummary, build_dashboard
from enhanced_terminal import print_enhanced_dashboard, export_daily_summary_csv
from csv_schema import ensure_csv_schema
from market_data_health import (
    MarketDataHealth,
    assess_market_data,
    assess_market_data_snapshot,
    event_time_to_monotonic,
)
from live_market_data import write_market_snapshot
from market_history import get_market_store
from orderbook_execution import ExecutionEstimate, fetch_public_execution_estimate
from paper_simulator import PaperSimulator
from reconnect import ExponentialBackoff, run_with_reconnect
from security_and_fees import (
    DEFAULT_ESTIMATED_FEES_PERCENT,
    check_security_guardrails,
    calculate_net_spread_safe,
    parse_positive_price,
    request_error_summary,
    csv_safe_text,
)

market_store = get_market_store()

console = Console()

try:
    CONFIG = load_config()
except ValueError as error:
    console.print(f"[bold red]Configuration invalide : {error}[/bold red]")
    raise SystemExit(2) from error

LM_URL = CONFIG.lm_url
TRADING_MODE = CONFIG.trading_mode
MODEL_NAME = CONFIG.model_name
MIN_SPREAD_THRESHOLD = CONFIG.min_spread_threshold
SPREAD_LOG_INTERVAL_SECONDS = CONFIG.spread_log_interval_seconds
DASHBOARD_INTERVAL_SECONDS = CONFIG.dashboard_interval_seconds
ALERT_CONFIRMATION_SECONDS = CONFIG.alert_confirmation_seconds
ALERT_REARM_HYSTERESIS_PERCENT = CONFIG.alert_rearm_hysteresis_percent
MARKET_DATA_STALE_SECONDS = CONFIG.market_data_stale_seconds
MARKET_DATA_MAX_SKEW_SECONDS = CONFIG.market_data_max_skew_seconds
MAX_ALLOWED_SPREAD_PERCENT = CONFIG.max_allowed_spread_percent
ESTIMATED_FEES = DEFAULT_ESTIMATED_FEES_PERCENT
PRICE_ALERT_COOLDOWN_SECONDS = CONFIG.price_alert_cooldown_seconds
NTFY_GLOBAL_COOLDOWN_SECONDS = CONFIG.ntfy_global_cooldown_seconds
NTFY_INITIAL_BACKOFF_SECONDS = 60.0
NTFY_MAX_BACKOFF_SECONDS = 900.0
SYMBOLS = ['BTC/USDT', 'ETH/USDT', 'SOL/USDT']
BINANCE_SYMBOLS = {symbol.replace('/', ''): symbol for symbol in SYMBOLS}
LOG_FILE = "arbitrage_log.csv"
LOG_HEADERS = ["Timestamp", "Symbol", "Kraken_Price", "Binance_Price", "Spread_Percent", "Net_Spread_Estimated", "LLM_Analysis"]
SPREAD_LOG_FILE = "spread_observations.csv"
PAPER_EXECUTION_FILE = "paper_execution_estimates.csv"
PAPER_EXECUTION_HEADERS = [
    "Timestamp_UTC",
    "Symbol",
    "Status",
    "Reason",
    "Buy_Exchange",
    "Buy_Amount",
    "Buy_VWAP_USDT",
    "Buy_Slippage_Percent",
    "Sell_Exchange",
    "Sell_Amount",
    "Sell_VWAP_USDT",
    "Sell_Slippage_Percent",
    "Gross_PnL_USDT",
    "Fees_USDT",
    "Net_PnL_USDT",
    "Inventory_Verified",
]
SPREAD_LOG_HEADERS = [
    "Timestamp_UTC",
    "Symbol",
    "Kraken_Price",
    "Binance_Price",
    "Gross_Spread_Percent",
    "Estimated_Net_Spread_Percent",
    "Minimum_Trigger_Threshold_Percent",
    "Estimated_Fees_Percent",
    "Threshold_Reached",
    "Net_Spread_Positive",
    "Opportunity_Triggered",
    "Market_Data_Status",
    "Kraken_Age_Seconds",
    "Binance_Age_Seconds",
    "Exchange_Time_Skew_Seconds",
]
NTFY_TOPIC_URL = CONFIG.ntfy_topic_url
MARKET_SNAPSHOT_FILE = os.environ.get("MARKET_SNAPSHOT_FILE", "market_snapshot.json")

if not os.path.exists(LOG_FILE):
    with open(LOG_FILE, mode='w', newline='', encoding='utf-8') as f:
        writer = csv.writer(f)
        writer.writerow(LOG_HEADERS)

ensure_csv_schema(
    SPREAD_LOG_FILE,
    SPREAD_LOG_HEADERS,
    added_column_defaults={
        "Market_Data_Status": "LEGACY",
        "Kraken_Age_Seconds": "",
        "Binance_Age_Seconds": "",
        "Exchange_Time_Skew_Seconds": "",
    },
)
ensure_csv_schema(PAPER_EXECUTION_FILE, PAPER_EXECUTION_HEADERS)

paper_simulator = PaperSimulator(
    mode=CONFIG.paper_mode,
    starting_balance_usdt=CONFIG.paper_starting_balance_usdt,
    position_percent=CONFIG.paper_position_percent,
    fixed_position_usdt=CONFIG.paper_fixed_position_usdt,
    exit_spread_percent=CONFIG.paper_exit_spread_percent,
    max_hold_seconds=CONFIG.paper_max_hold_seconds,
    estimated_fees_percent=ESTIMATED_FEES,
)

latest_prices = {"Kraken": {}, "Binance": {}}
last_price_update_at = {"Kraken": {}, "Binance": {}}
price_update_generation = {"Kraken": {}, "Binance": {}}
last_movement_notification_at = {}
last_spread_log_at = {}
spread_summaries = {symbol: SpreadSummary() for symbol in SYMBOLS}
alert_gate = AlertGate(ALERT_CONFIRMATION_SECONDS)
price_updates_count = 0
opportunities_count = 0
ntfy_lock = threading.Lock()
spread_log_lock = threading.Lock()
paper_execution_lock = threading.Lock()
ntfy_blocked_until = 0.0
ntfy_backoff_seconds = NTFY_INITIAL_BACKOFF_SECONDS

def ask_llm_analysis(symbol, price_kraken, price_binance, spread):
    if not CONFIG.lm_enabled:
        return "Analyse LLM désactivée"
    prompt = f"Analyse le spread d'arbitrage pour {symbol}. Kraken: {price_kraken}, Binance: {price_binance}, Écart brut: {spread:.2f}%. Est-ce une opportunité valide après frais d'échange ? Réponds brièvement."
    try:
        parsed = urlsplit(LM_URL)
        res = secure_request(
            f"{LM_URL}/chat/completions",
            method="POST", allowed_hosts={parsed.hostname}, local_only=True,
            allowed_ports={parsed.port or 80}, max_bytes=1024 * 1024,
            json={
                "model": MODEL_NAME,
                "messages": [{"role": "user", "content": prompt}],
                "temperature": 0.2,
                "max_tokens": 500,
            },
            timeout=30
        )
        if res.status_code == 200:
            content = res.json()['choices'][0]['message']['content']
            if not isinstance(content, str):
                raise ValueError("Invalid model response")
            return csv_safe_text(content)
    except (SecureHTTPError, ValueError, KeyError, IndexError, TypeError):
        return "Analyse LLM indisponible"
    return "Pas de réponse du modèle"

def log_opportunity(symbol, p_kraken, p_binance, spread, net_spread, analysis):
    with open(LOG_FILE, mode='a', newline='', encoding='utf-8') as f:
        writer = csv.writer(f)
        writer.writerow([time.strftime('%Y-%m-%d %H:%M:%S'), symbol, p_kraken, p_binance, f"{spread:.4f}%", f"{net_spread:.4f}%", csv_safe_text(analysis)])

def log_spread_observation(
    symbol,
    price_kraken,
    price_binance,
    spread,
    net_spread,
    threshold_reached,
    opportunity_triggered,
    market_health,
):
    row = {
        "Timestamp_UTC": datetime.now(timezone.utc).isoformat(timespec="milliseconds").replace("+00:00", "Z"),
        "Symbol": symbol,
        "Kraken_Price": price_kraken,
        "Binance_Price": price_binance,
        "Gross_Spread_Percent": spread,
        "Estimated_Net_Spread_Percent": net_spread,
        "Minimum_Trigger_Threshold_Percent": MIN_SPREAD_THRESHOLD,
        "Estimated_Fees_Percent": ESTIMATED_FEES,
        "Threshold_Reached": str(threshold_reached).lower(),
        "Net_Spread_Positive": str(net_spread > 0).lower(),
        "Opportunity_Triggered": str(opportunity_triggered).lower(),
        "Market_Data_Status": market_health.status,
        "Kraken_Age_Seconds": market_health.kraken_age_seconds,
        "Binance_Age_Seconds": market_health.binance_age_seconds,
        "Exchange_Time_Skew_Seconds": market_health.exchange_skew_seconds,
    }
    with spread_log_lock:
        with open(SPREAD_LOG_FILE, mode='a', newline='', encoding='utf-8') as f:
            writer = csv.DictWriter(f, fieldnames=SPREAD_LOG_HEADERS)
            writer.writerow(row)

def log_paper_execution_estimate(symbol, estimate, *, reason=None):
    buy = estimate.buy
    sell = estimate.sell
    row = [
        datetime.now(timezone.utc).isoformat(timespec="milliseconds").replace("+00:00", "Z"),
        symbol,
        estimate.status,
        reason or estimate.reason,
        buy.exchange if buy else "",
        f"{buy.amount:.10f}" if buy else "",
        f"{buy.average_price:.10f}" if buy else "",
        f"{buy.slippage_percent:.8f}" if buy else "",
        sell.exchange if sell else "",
        f"{sell.amount:.10f}" if sell else "",
        f"{sell.average_price:.10f}" if sell else "",
        f"{sell.slippage_percent:.8f}" if sell else "",
        f"{estimate.gross_pnl:.8f}",
        f"{estimate.fees:.8f}",
        f"{estimate.net_pnl:.8f}",
        str(estimate.inventory_verified).lower(),
    ]
    with paper_execution_lock:
        with open(PAPER_EXECUTION_FILE, mode="a", newline="", encoding="utf-8") as target:
            writer = csv.writer(target)
            writer.writerow(row)

def retry_after_seconds(value):
    if not value:
        return 0.0
    try:
        parsed = float(value)
        return min(NTFY_MAX_BACKOFF_SECONDS, max(0.0, parsed)) if math.isfinite(parsed) else 0.0
    except ValueError:
        try:
            retry_at = parsedate_to_datetime(value)
            if retry_at.tzinfo is None:
                retry_at = retry_at.replace(tzinfo=timezone.utc)
            return max(0.0, (retry_at - datetime.now(timezone.utc)).total_seconds())
        except (TypeError, ValueError, OverflowError):
            return 0.0

def send_ntfy_notification(title, message, *, pre_send_check=None):
    global ntfy_blocked_until, ntfy_backoff_seconds

    if not NTFY_TOPIC_URL:
        return False

    with ntfy_lock:
        now = time.monotonic()
        if now < ntfy_blocked_until:
            return False
        if pre_send_check is not None and not pre_send_check():
            return False

        try:
            response = secure_request(
                NTFY_TOPIC_URL,
                method="POST", allowed_hosts={urlsplit(NTFY_TOPIC_URL).hostname},
                max_bytes=64 * 1024,
                data=message.encode('utf-8'),
                headers={"Title": title},
                timeout=5
            )
        except (requests.RequestException, SecureHTTPError) as e:
            ntfy_blocked_until = time.monotonic() + NTFY_GLOBAL_COOLDOWN_SECONDS
            console.print(
                f"  ⚠️ Échec de la notification ntfy : "
                f"{escape(request_error_summary(e))}"
            )
            return False

        if response.status_code == 429:
            server_retry_after = retry_after_seconds(response.headers.get("Retry-After"))
            delay = max(ntfy_backoff_seconds, server_retry_after)
            ntfy_blocked_until = time.monotonic() + delay
            ntfy_backoff_seconds = min(ntfy_backoff_seconds * 2, NTFY_MAX_BACKOFF_SECONDS)
            print(f"  ⚠️ ntfy limite les envois; notifications suspendues pendant {delay:.0f}s.")
            return False

        try:
            response.raise_for_status()
        except (requests.RequestException, SecureHTTPError) as e:
            ntfy_blocked_until = time.monotonic() + NTFY_GLOBAL_COOLDOWN_SECONDS
            console.print(
                f"  ⚠️ Échec de la notification ntfy : "
                f"{escape(request_error_summary(e))}"
            )
            return False

        ntfy_blocked_until = time.monotonic() + NTFY_GLOBAL_COOLDOWN_SECONDS
        ntfy_backoff_seconds = NTFY_INITIAL_BACKOFF_SECONDS
        return True

def notify_opportunity(symbol, net_spread, market_data_snapshot=None):
    return send_ntfy_notification(
        "Pravda-Bot Opportunity",
        f"Spread net : {net_spread:.2f}% sur {symbol}",
        pre_send_check=(
            lambda: is_market_data_snapshot_fresh(market_data_snapshot, symbol)
        ) if market_data_snapshot is not None else None,
    )

def notify_price_movement(
    exchange_name,
    symbol,
    previous_price,
    current_price,
    change_pct,
    market_data_snapshot=None,
):
    return send_ntfy_notification(
        "Pravda-Bot Price Movement",
        f"{exchange_name} {symbol}: {previous_price} -> {current_price} ({change_pct:+.4f}%)",
        pre_send_check=(
            lambda: is_market_data_snapshot_fresh(market_data_snapshot, symbol)
        ) if market_data_snapshot is not None else None,
    )


def current_market_data_health(snapshot, symbol):
    return assess_market_data_snapshot(
        snapshot,
        (
            price_update_generation["Kraken"].get(symbol),
            price_update_generation["Binance"].get(symbol),
        ),
        last_price_update_at["Kraken"].get(symbol),
        last_price_update_at["Binance"].get(symbol),
        stale_after_seconds=MARKET_DATA_STALE_SECONDS,
        max_skew_seconds=MARKET_DATA_MAX_SKEW_SECONDS,
    )


def is_market_data_snapshot_fresh(snapshot, symbol):
    health = current_market_data_health(snapshot, symbol)
    return health is not None and health.status == "OK"


def run_market_data_action(snapshot, symbol, action, *args, **kwargs):
    health = current_market_data_health(snapshot, symbol)
    if health is None or health.status != "OK":
        return False, None
    return True, action(*args, **kwargs)


MARKET_SNAPSHOT_INTERVAL_SECONDS = 0.5


async def publish_market_snapshot():
    # Capture together on the loop; threads never receive mutable live state.
    prices = {exchange: dict(values) for exchange, values in latest_prices.items()}
    timestamps = {exchange: dict(values) for exchange, values in last_price_update_at.items()}
    captured_mono = time.monotonic()
    captured_utc = datetime.now(timezone.utc)
    return await asyncio.to_thread(
        write_market_snapshot, prices, timestamps, path=MARKET_SNAPSHOT_FILE,
        now_monotonic=captured_mono, now_utc=captured_utc,
    )


async def publish_market_snapshots_periodically():
    # A single writer, independent of the three workers, with one write in flight.
    while True:
        try:
            prices = {exchange: dict(values) for exchange, values in latest_prices.items()}
            timestamps = {exchange: dict(values) for exchange, values in last_price_update_at.items()}
            captured_mono = time.monotonic()
            captured_utc = datetime.now(timezone.utc)
            await asyncio.to_thread(write_market_snapshot, prices, timestamps, path=MARKET_SNAPSHOT_FILE,
                now_monotonic=captured_mono, now_utc=captured_utc)
        except OSError as error:
            console.print(
                f"[bold red]Échec de publication des cotations dashboard "
                f"({type(error).__name__}).[/bold red]"
            )
        await asyncio.sleep(MARKET_SNAPSHOT_INTERVAL_SECONDS)


@dataclass(frozen=True)
class PriceUpdate:
    exchange_name: str
    symbol: str
    previous_price: float | None
    price: float
    price_kraken: float | None
    price_binance: float | None
    market_data_snapshot: tuple
    kraken_updated_at: float | None
    binance_updated_at: float | None
    event_at: datetime


class MarketUpdateDispatcher:
    """Three workers and one replaceable snapshot per symbol; no per-tick tasks.

    SQLite records selected samples with their exchange event time. Intermediate
    ticks replaced while a worker is busy are not an exhaustive tick archive.
    """
    def __init__(self):
        self.pending = {}
        self.events = {symbol: asyncio.Event() for symbol in SYMBOLS}
        self.workers = [asyncio.create_task(self._work(symbol)) for symbol in SYMBOLS]
        self.users = 0
        self.closed = False

    def submit(self, update):
        if update is not None and not self.closed:
            self.pending[update.symbol] = update
            self.events[update.symbol].set()

    async def _work(self, symbol):
        while True:
            await self.events[symbol].wait()
            self.events[symbol].clear()
            update = self.pending.pop(symbol, None)
            if update is None:
                continue
            try:
                await process_price_update(update)
            except asyncio.CancelledError:
                raise
            except Exception as error:
                alert_gate.reset_candidate(symbol)
                console.print(
                    f"[bold red]Traitement {symbol} interrompu "
                    f"({type(error).__name__}).[/bold red]"
                )

    async def close(self):
        self.closed = True
        self.pending.clear()
        for worker in self.workers:
            worker.cancel()
        await asyncio.gather(*self.workers, return_exceptions=True)


_feed_dispatcher = None


@asynccontextmanager
async def feed_dispatcher_scope():
    global _feed_dispatcher
    if _feed_dispatcher is None:
        _feed_dispatcher = MarketUpdateDispatcher()
    dispatcher = _feed_dispatcher
    dispatcher.users += 1
    try:
        yield dispatcher
    finally:
        dispatcher.users -= 1
        if dispatcher.users == 0:
            _feed_dispatcher = None
            await dispatcher.close()


def invalidate_exchange(exchange_name):
    for symbol in SYMBOLS:
        price_update_generation[exchange_name][symbol] = (
            price_update_generation[exchange_name].get(symbol, 0) + 1
        )
        last_price_update_at[exchange_name].pop(symbol, None)
        alert_gate.reset_candidate(symbol)
    if _feed_dispatcher is not None:
        _feed_dispatcher.pending.clear()


def record_price_update(exchange_name, symbol, raw_price, event_timestamp=None):
    """Validate and capture price, event time and generation without awaiting I/O."""
    global price_updates_count
    if exchange_name not in latest_prices or symbol not in SYMBOLS:
        return None
    received_mono = time.monotonic()
    received_utc = datetime.now(timezone.utc)
    previous_health = assess_market_data(
        last_price_update_at["Kraken"].get(symbol), last_price_update_at["Binance"].get(symbol),
        now=received_mono, stale_after_seconds=MARKET_DATA_STALE_SECONDS,
        max_skew_seconds=MARKET_DATA_MAX_SKEW_SECONDS,
    )
    if previous_health.status != "OK":
        alert_gate.reset_candidate(symbol)

    try:
        price = parse_positive_price(raw_price)
    except ValueError:
        price_update_generation[exchange_name][symbol] = (
            price_update_generation[exchange_name].get(symbol, 0) + 1
        )
        alert_gate.reset_candidate(symbol)
        console.print(
            f"[bold yellow][GARDE-FOU] Prix invalide reçu de "
            f"{exchange_name} pour {symbol}; mise à jour ignorée.[/bold yellow]"
        )
        return

    try:
        updated_at = (
            received_mono
            if event_timestamp is None
            else event_time_to_monotonic(
                event_timestamp, now_epoch=received_utc.timestamp(), now_monotonic=received_mono
            )
        )
    except ValueError as error:
        price_update_generation[exchange_name][symbol] = (
            price_update_generation[exchange_name].get(symbol, 0) + 1
        )
        alert_gate.reset_candidate(symbol)
        console.print(
            f"[bold yellow][GARDE-FOU] Horodatage invalide reçu de "
            f"{exchange_name} pour {symbol} ({type(error).__name__}); mise à jour ignorée.[/bold yellow]"
        )
        return

    previous_price = latest_prices[exchange_name].get(symbol)
    latest_prices[exchange_name][symbol] = price
    last_price_update_at[exchange_name][symbol] = updated_at
    price_update_generation[exchange_name][symbol] = (
        price_update_generation[exchange_name].get(symbol, 0) + 1
    )
    price_updates_count += 1
    price_kraken = latest_prices["Kraken"].get(symbol)
    price_binance = latest_prices["Binance"].get(symbol)
    market_data_snapshot = (
        price_update_generation["Kraken"].get(symbol),
        price_update_generation["Binance"].get(symbol),
    )
    kraken_updated_at = last_price_update_at["Kraken"].get(symbol)
    binance_updated_at = last_price_update_at["Binance"].get(symbol)
    health = assess_market_data(
        kraken_updated_at, binance_updated_at, now=received_mono,
        stale_after_seconds=MARKET_DATA_STALE_SECONDS,
        max_skew_seconds=MARKET_DATA_MAX_SKEW_SECONDS,
    )
    if price_kraken is None or price_binance is None or health.status != "OK":
        alert_gate.reset_candidate(symbol)
    else:
        spread = abs(price_kraken - price_binance) / min(price_kraken, price_binance) * 100
        if not math.isfinite(spread) or spread > MAX_ALLOWED_SPREAD_PERCENT:
            alert_gate.reset_candidate(symbol)
        elif spread < MIN_SPREAD_THRESHOLD or spread <= ESTIMATED_FEES:
            # Preserve confirmation interruptions and rearming even when coalesced.
            alert_gate.update(
                symbol, profitable=False, spread=spread,
                rearm_threshold=max(0.0, max(MIN_SPREAD_THRESHOLD, ESTIMATED_FEES)
                                    - ALERT_REARM_HYSTERESIS_PERCENT), now=received_mono,
            )
    event_at = datetime.fromtimestamp(
        received_utc.timestamp() - (received_mono - updated_at), timezone.utc
    )
    return PriceUpdate(
        exchange_name, symbol, previous_price, price, price_kraken, price_binance,
        market_data_snapshot, kraken_updated_at, binance_updated_at, event_at,
    )


async def handle_price_update(exchange_name, symbol, raw_price, event_timestamp=None):
    """Direct API; WebSocket readers submit the immutable update to shared workers."""
    update = record_price_update(exchange_name, symbol, raw_price, event_timestamp)
    if update is not None:
        await process_price_update(update)


async def process_price_update(update):
    global opportunities_count
    exchange_name, symbol = update.exchange_name, update.symbol
    previous_price, price = update.previous_price, update.price
    price_kraken, price_binance = update.price_kraken, update.price_binance
    market_data_snapshot = update.market_data_snapshot
    kraken_updated_at, binance_updated_at = update.kraken_updated_at, update.binance_updated_at
    try:
        await asyncio.to_thread(
            market_store.record_tick, symbol, exchange_name, price, timestamp=update.event_at
        )
    except (OSError, sqlite3.Error, ValueError) as error:
        console.print(
            f"[bold red]Échec d’enregistrement de l’historique marché "
            f"({type(error).__name__}).[/bold red]"
        )
    if current_market_data_health(market_data_snapshot, symbol) is None:
        return

    if price_kraken is None or price_binance is None:
        return

    now = time.monotonic()
    market_health = assess_market_data(
        kraken_updated_at,
        binance_updated_at,
        stale_after_seconds=MARKET_DATA_STALE_SECONDS,
        max_skew_seconds=MARKET_DATA_MAX_SKEW_SECONDS,
    )
    if market_health.status != "OK":
        alert_gate.reset_candidate(symbol)
        spread = abs(price_kraken - price_binance) / min(price_kraken, price_binance) * 100
        if not math.isfinite(spread):
            console.print(
                f"[bold yellow]Spread non fini ignoré pour {symbol} "
                f"(état du flux : {market_health.status}).[/bold yellow]"
            )
            return
        net_spread = calculate_net_spread_safe(
            price_kraken,
            price_binance,
            ESTIMATED_FEES / 100,
        ) * 100
        last_logged_at = last_spread_log_at.get(symbol)
        if last_logged_at is None or now - last_logged_at >= SPREAD_LOG_INTERVAL_SECONDS:
            last_spread_log_at[symbol] = now
            try:
                await asyncio.to_thread(
                    log_spread_observation,
                    symbol,
                    price_kraken,
                    price_binance,
                    spread,
                    net_spread,
                    spread >= MIN_SPREAD_THRESHOLD,
                    False,
                    market_health,
                )
            except OSError as error:
                console.print(
                    f"[bold red]Échec de journalisation du spread : {error}[/bold red]"
                )
        return

    spread = abs(price_kraken - price_binance) / min(price_kraken, price_binance) * 100
    if spread != 0 and not check_security_guardrails(
        spread, MAX_ALLOWED_SPREAD_PERCENT
    ):
        alert_gate.reset_candidate(symbol)
        if not math.isfinite(spread):
            console.print(
                f"[bold yellow]Spread non fini ignoré pour {symbol}; "
                "aucune simulation ni alerte arbitrage ne sera déclenchée.[/bold yellow]"
            )
            return
        net_spread = calculate_net_spread_safe(
            price_kraken,
            price_binance,
            ESTIMATED_FEES / 100,
        ) * 100
        anomaly_health = MarketDataHealth(
            "ANOMALY",
            market_health.kraken_age_seconds,
            market_health.binance_age_seconds,
            market_health.exchange_skew_seconds,
        )
        last_logged_at = last_spread_log_at.get(symbol)
        if last_logged_at is None or now - last_logged_at >= SPREAD_LOG_INTERVAL_SECONDS:
            last_spread_log_at[symbol] = now
            try:
                await asyncio.to_thread(
                    log_spread_observation,
                    symbol,
                    price_kraken,
                    price_binance,
                    spread,
                    net_spread,
                    spread >= MIN_SPREAD_THRESHOLD,
                    False,
                    anomaly_health,
                )
            except OSError as error:
                console.print(
                    f"[bold red]Échec de journalisation du spread anormal : {error}[/bold red]"
                )
        return

    net_spread = calculate_net_spread_safe(
        price_kraken,
        price_binance,
        ESTIMATED_FEES / 100,
    ) * 100
    if previous_price is not None and price != previous_price:
        change_pct = (price - previous_price) / previous_price * 100
        if not math.isfinite(change_pct):
            console.print(
                f"[bold yellow][GARDE-FOU] Variation non finie ignorée pour "
                f"{exchange_name} {symbol}.[/bold yellow]"
            )
        elif now - last_movement_notification_at.get(symbol, 0) >= PRICE_ALERT_COOLDOWN_SECONDS:
            sent, _ = await asyncio.to_thread(
                run_market_data_action, market_data_snapshot, symbol, notify_price_movement,
                exchange_name, symbol, previous_price, price, change_pct, market_data_snapshot,
            )
            if sent:
                last_movement_notification_at[symbol] = time.monotonic()
    if not is_market_data_snapshot_fresh(market_data_snapshot, symbol):
        return
    threshold_reached = spread >= MIN_SPREAD_THRESHOLD
    is_profitable = threshold_reached and net_spread > 0
    rearm_spread = max(
        0.0,
        max(MIN_SPREAD_THRESHOLD, ESTIMATED_FEES)
        - ALERT_REARM_HYSTERESIS_PERCENT,
    )
    opportunity_triggered = alert_gate.update(
        symbol,
        profitable=is_profitable,
        spread=spread,
        rearm_threshold=rearm_spread,
        now=now,
    )
    activation_id = alert_gate.activation_id(symbol) if opportunity_triggered else None

    last_logged_at = last_spread_log_at.get(symbol)
    if (
        last_logged_at is None
        or now - last_logged_at >= SPREAD_LOG_INTERVAL_SECONDS
        or opportunity_triggered
    ):
        last_spread_log_at[symbol] = now
        if not opportunity_triggered:
            try:
                await asyncio.to_thread(
                    log_spread_observation,
                    symbol,
                    price_kraken,
                    price_binance,
                    spread,
                    net_spread,
                    threshold_reached,
                    False,
                    market_health,
                )
            except OSError as e:
                console.print(f"[bold red]Échec de journalisation du spread : {e}[/bold red]")

    current_market_health = current_market_data_health(market_data_snapshot, symbol)
    if current_market_health is None:
        if activation_id is not None:
            alert_gate.cancel_activation(symbol, activation_id)
        return
    if current_market_health.status != "OK":
        alert_gate.reset_candidate(symbol)
        if activation_id is not None:
            alert_gate.cancel_activation(symbol, activation_id)
        console.print(
            f"[bold yellow]Signal ignoré pour {symbol} : les prix sont devenus "
            f"{current_market_health.status} pendant la journalisation.[/bold yellow]"
        )
        return

    if not opportunity_triggered:
        spread_summaries[symbol].record(spread, net_spread, threshold_reached, False)

    try:
        paper_result = paper_simulator.update(
            symbol,
            price_kraken,
            price_binance,
            now=datetime.now(timezone.utc),
        )
    except OSError as error:
        console.print(f"[bold red]Échec du journal paper : {error}[/bold red]")
        paper_result = None
    if paper_result is not None and paper_result["action"] == "closed":
        console.print(
            f"[cyan][PAPER] Position fermée pour {symbol} : "
            f"{paper_result['net_pnl_usdt']:+.2f} USDT estimés "
            f"({paper_result['reason']}).[/cyan]"
        )

    if not opportunity_triggered:
        return

    execution_estimate = None
    execution_error = None
    try:
        execution_started, execution_estimate = await asyncio.to_thread(
            run_market_data_action,
            market_data_snapshot,
            symbol,
            fetch_public_execution_estimate,
            symbol,
            target_notional=paper_simulator.next_position_notional_usdt,
            taker_fee_percent=CONFIG.paper_taker_fee_percent,
            max_slippage_percent=CONFIG.paper_max_slippage_percent,
            max_book_age_seconds=CONFIG.paper_orderbook_max_age_seconds,
        )
        if not execution_started or not is_market_data_snapshot_fresh(market_data_snapshot, symbol):
            if activation_id is not None:
                alert_gate.cancel_activation(symbol, activation_id)
            return
        await asyncio.to_thread(
            run_market_data_action, market_data_snapshot, symbol,
            log_paper_execution_estimate, symbol, execution_estimate,
        )
    except (ccxt.BaseError, OSError, ValueError) as error:
        execution_error = (
            str(error) if isinstance(error, ValueError) else request_error_summary(error)
        )
        unavailable_estimate = ExecutionEstimate(
            status="unavailable",
            reason=execution_error,
        )
        try:
            await asyncio.to_thread(
                run_market_data_action,
                market_data_snapshot,
                symbol,
                log_paper_execution_estimate,
                symbol,
                unavailable_estimate,
            )
        except OSError as log_error:
            console.print(
                f"[bold red]Échec de journalisation de l’estimation d’exécution : "
                f"{log_error}[/bold red]"
            )
        console.print(
            f"[bold yellow]Simulation d’exécution indisponible pour {symbol} : "
            f"{escape(execution_error)}. Aucun trade papier ouvert.[/bold yellow]"
        )

    latest_execution_health = current_market_data_health(market_data_snapshot, symbol)
    if latest_execution_health is None or latest_execution_health.status != "OK":
        alert_gate.reset_candidate(symbol)
        if activation_id is not None:
            alert_gate.cancel_activation(symbol, activation_id)
        return
    opportunities_count += 1
    spread_summaries[symbol].record(spread, net_spread, threshold_reached, True)
    execution_approved = (
        execution_estimate is not None
        and execution_estimate.status == "estimate_only"
        and latest_execution_health is not None
        and latest_execution_health.status == "OK"
    )
    if execution_estimate is not None:
        console.print(
            f"[cyan][CARNET] {symbol} : {execution_estimate.status} — "
            f"{execution_estimate.reason}; PnL net estimé "
            f"{execution_estimate.net_pnl:+.4f} USDT. "
            "Inventaires des comptes non vérifiés; estimation papier uniquement.[/cyan]"
        )
    elif latest_execution_health is None or latest_execution_health.status != "OK":
        console.print(
            f"[bold yellow]Signal papier ignoré pour {symbol} : les cotations "
            "sont devenues obsolètes pendant l’analyse des carnets.[/bold yellow]"
        )

    try:
        paper_result = paper_simulator.update(
            symbol,
            price_kraken,
            price_binance,
            opportunity_confirmed=True,
            execution_approved=execution_approved,
            now=datetime.now(timezone.utc),
        )
    except OSError as error:
        console.print(f"[bold red]Échec du journal paper : {error}[/bold red]")
        paper_result = None
    console.print(
        f"[bold green]🎯 Signal confirmé : {symbol} "
        f"({net_spread:.4f}% net estimé)[/bold green]"
    )
    if paper_result is not None and paper_result["action"] == "opened":
        console.print(
            f"[cyan][PAPER] Position virtuelle ouverte : "
            f"{paper_result['notional_usdt']:.2f} USDT sur {symbol} "
            f"(aucun ordre envoyé; soldes d’échange non vérifiés).[/cyan]"
        )
    try:
        await asyncio.to_thread(
            run_market_data_action,
            market_data_snapshot,
            symbol,
            log_spread_observation,
            symbol,
            price_kraken,
            price_binance,
            spread,
            net_spread,
            threshold_reached,
            True,
            market_health,
        )
    except OSError as error:
        console.print(f"[bold red]Échec de journalisation du signal : {error}[/bold red]")

    notification_started, _ = await asyncio.to_thread(
        run_market_data_action,
        market_data_snapshot,
        symbol,
        notify_opportunity,
        symbol,
        net_spread,
        market_data_snapshot,
    )
    if not notification_started:
        if activation_id is not None:
            alert_gate.cancel_activation(symbol, activation_id)
        console.print(
            f"[bold yellow]Notification ignorée pour {symbol} : les prix "
            "ont changé ou sont devenus obsolètes avant son exécution.[/bold yellow]"
        )
        return

    current_market_health = current_market_data_health(market_data_snapshot, symbol)
    if current_market_health is None or current_market_health.status != "OK":
        console.print(
            f"[bold yellow]Analyse LLM ignorée pour {symbol} : les prix "
            "ont changé ou sont devenus obsolètes après la notification.[/bold yellow]"
        )
        return
    analysis_started, analysis = await asyncio.to_thread(
        run_market_data_action,
        market_data_snapshot,
        symbol,
        ask_llm_analysis,
        symbol,
        price_kraken,
        price_binance,
        spread,
    )
    if not analysis_started:
        console.print(
            f"[bold yellow]Analyse LLM ignorée pour {symbol} : les prix "
            "ont changé ou sont devenus obsolètes avant son exécution.[/bold yellow]"
        )
        return
    if not is_market_data_snapshot_fresh(market_data_snapshot, symbol):
        return
    console.print(f"  [green]🤖 LLM: {escape(analysis)}[/green]")
    await asyncio.to_thread(
        run_market_data_action, market_data_snapshot, symbol, log_opportunity,
        symbol, price_kraken, price_binance, spread, net_spread, analysis,
    )

async def stream_kraken_tickers():
    async with feed_dispatcher_scope() as dispatcher:
        try:
            async with websockets.connect("wss://ws.kraken.com/v2", ping_interval=20, ping_timeout=20, open_timeout=15) as websocket:
                await websocket.send(json.dumps({
                    "method": "subscribe",
                    "params": {"channel": "ticker", "symbol": SYMBOLS,
                               "event_trigger": "trades", "snapshot": True},
                    "req_id": 1,
                }))
                async for raw_message in websocket:
                    message = json.loads(raw_message)
                    if message.get("channel") != "ticker":
                        continue
                    for ticker in message.get("data", []):
                        symbol = ticker.get("symbol")
                        if symbol in SYMBOLS and ticker.get("last") is not None:
                            dispatcher.submit(record_price_update(
                                "Kraken", symbol, ticker["last"], ticker.get("timestamp")))
        finally:
            invalidate_exchange("Kraken")

async def stream_binance_tickers():
    streams = "/".join(f"{symbol.replace('/', '').lower()}@ticker" for symbol in SYMBOLS)
    url = f"wss://stream.binance.com:9443/stream?streams={streams}"
    async with feed_dispatcher_scope() as dispatcher:
        try:
            async with websockets.connect(url, ping_interval=20, ping_timeout=20, open_timeout=15) as websocket:
                async for raw_message in websocket:
                    message = json.loads(raw_message)
                    ticker = message.get("data", message)
                    symbol = BINANCE_SYMBOLS.get(ticker.get("s"))
                    if symbol is not None and ticker.get("c") is not None and ticker.get("E") is not None:
                        dispatcher.submit(record_price_update("Binance", symbol, ticker["c"], ticker["E"]))
        finally:
            invalidate_exchange("Binance")

async def run_stream_with_reconnect(exchange_name, stream_function):
    await run_with_reconnect(
        stream_function,
        exchange_name,
        backoff=ExponentialBackoff(base_delay=1.0, max_delay=30.0),
    )

async def render_dashboard_periodically():
    counter = 0
    while True:
        await asyncio.sleep(DASHBOARD_INTERVAL_SECONDS)
        console.print(
            build_dashboard(
                SYMBOLS,
                latest_prices,
                spread_summaries,
                threshold=MIN_SPREAD_THRESHOLD,
                updates=price_updates_count,
                opportunities=opportunities_count,
                last_price_update_at=last_price_update_at,
                stale_after_seconds=MARKET_DATA_STALE_SECONDS,
                max_skew_seconds=MARKET_DATA_MAX_SKEW_SECONDS,
                paper_simulator=paper_simulator,
            )
        )

        counter += 1
        if counter % 5 == 0:
            prices_snapshot = {name: dict(values) for name, values in latest_prices.items()}
            try:
                await asyncio.to_thread(print_enhanced_dashboard, prices_snapshot)
                await asyncio.to_thread(export_daily_summary_csv, "daily_summary.csv")
            except (OSError, ValueError, sqlite3.Error) as error:
                console.print(f"Terminal enrichi indisponible ({type(error).__name__})")

async def monitor_markets():
    print(f"=== Pravda-Bot Temps Réel (Mode: {TRADING_MODE}) ===")
    print(f"Paires surveillées : {', '.join(SYMBOLS)}")
    print(f"Seuil de déclenchement : {MIN_SPREAD_THRESHOLD:g}%")
    print(f"Spread maximum accepté : {MAX_ALLOWED_SPREAD_PERCENT:g}%")
    print(f"Échantillonnage des spreads : {SPREAD_LOG_INTERVAL_SECONDS:g}s par paire")
    print(f"Tableau de bord Rich : toutes les {DASHBOARD_INTERVAL_SECONDS:g}s")
    print(
        f"Confirmation d'alerte : {ALERT_CONFIRMATION_SECONDS:g}s; "
        f"hystérésis : {ALERT_REARM_HYSTERESIS_PERCENT:g}%"
    )
    print(
        f"Santé des flux : obsolète après {MARKET_DATA_STALE_SECONDS:g}s; "
        f"désynchronisation max {MARKET_DATA_MAX_SKEW_SECONDS:g}s"
    )
    print(
        f"Simulation paper : mode {CONFIG.paper_mode}; solde initial "
        f"{CONFIG.paper_starting_balance_usdt:.2f} USDT; sizing fixe "
        f"{CONFIG.paper_fixed_position_usdt:.2f} USDT ou "
        f"{CONFIG.paper_position_percent:g}% du solde selon le mode; "
        f"journal paper_trades.csv"
    )
    print(f"Observations CSV : {SPREAD_LOG_FILE}")
    print(
        "Notifications ntfy : configurées"
        if NTFY_TOPIC_URL
        else "Notifications ntfy : désactivées (NTFY_TOPIC_URL absent)"
    )
    print(f"Délai anti-rafale par paire : {PRICE_ALERT_COOLDOWN_SECONDS:g}s\n")
    print(f"Délai global ntfy : {NTFY_GLOBAL_COOLDOWN_SECONDS:g}s; backoff initial après 429 : {NTFY_INITIAL_BACKOFF_SECONDS:g}s\n")
    await asyncio.gather(
        run_stream_with_reconnect("Kraken", stream_kraken_tickers),
        run_stream_with_reconnect("Binance", stream_binance_tickers),
        publish_market_snapshots_periodically(),
        render_dashboard_periodically(),
    )

try:
    asyncio.run(monitor_markets())
except KeyboardInterrupt:
    print("\n=== Arrêt du bot demandé ===")
    print(f"Mises à jour de prix reçues : {price_updates_count}")
    print(f"Opportunités analysées : {opportunities_count}")
    print("Fermeture propre.")
