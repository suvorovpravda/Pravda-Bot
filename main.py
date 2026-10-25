import asyncio
import csv
import json
import math
import os
import threading
import time
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime

import requests
import websockets
from dotenv import load_dotenv
from rich.console import Console
from rich.markup import escape

from bot_config import load_config
from bot_dashboard import SpreadSummary, build_dashboard

load_dotenv()

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
ESTIMATED_FEES = 0.36
PRICE_ALERT_COOLDOWN_SECONDS = CONFIG.price_alert_cooldown_seconds
NTFY_GLOBAL_COOLDOWN_SECONDS = CONFIG.ntfy_global_cooldown_seconds
NTFY_INITIAL_BACKOFF_SECONDS = 60.0
NTFY_MAX_BACKOFF_SECONDS = 900.0
STREAM_RECONNECT_SECONDS = 5

SYMBOLS = ['BTC/USDT', 'ETH/USDT', 'SOL/USDT']
BINANCE_SYMBOLS = {symbol.replace('/', ''): symbol for symbol in SYMBOLS}
LOG_FILE = "arbitrage_log.csv"
LOG_HEADERS = ["Timestamp", "Symbol", "Kraken_Price", "Binance_Price", "Spread_Percent", "Net_Spread_Estimated", "LLM_Analysis"]
SPREAD_LOG_FILE = "spread_observations.csv"
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
]
NTFY_TOPIC_URL = CONFIG.ntfy_topic_url

if not os.path.exists(LOG_FILE):
    with open(LOG_FILE, mode='w', newline='', encoding='utf-8') as f:
        writer = csv.writer(f)
        writer.writerow(LOG_HEADERS)

if not os.path.exists(SPREAD_LOG_FILE) or os.path.getsize(SPREAD_LOG_FILE) == 0:
    with open(SPREAD_LOG_FILE, mode='w', newline='', encoding='utf-8') as f:
        writer = csv.writer(f)
        writer.writerow(SPREAD_LOG_HEADERS)

latest_prices = {"Kraken": {}, "Binance": {}}
last_movement_notification_at = {}
last_spread_log_at = {}
spread_summaries = {symbol: SpreadSummary() for symbol in SYMBOLS}
last_profitable_state = {symbol: False for symbol in SYMBOLS}
price_updates_count = 0
opportunities_count = 0
ntfy_lock = threading.Lock()
spread_log_lock = threading.Lock()
ntfy_blocked_until = 0.0
ntfy_backoff_seconds = NTFY_INITIAL_BACKOFF_SECONDS

def ask_llm_analysis(symbol, price_kraken, price_binance, spread):
    prompt = f"Analyse le spread d'arbitrage pour {symbol}. Kraken: {price_kraken}, Binance: {price_binance}, Écart brut: {spread:.2f}%. Est-ce une opportunité valide après frais d'échange ? Réponds brièvement."
    try:
        res = requests.post(
            f"{LM_URL}/chat/completions",
            json={
                "model": MODEL_NAME,
                "messages": [{"role": "user", "content": prompt}],
                "temperature": 0.2
            },
            timeout=30
        )
        if res.status_code == 200:
            return res.json()['choices'][0]['message']['content'].replace('\n', ' ')
    except Exception as e:
        return f"Erreur LLM : {e}"
    return "Pas de réponse du modèle"

def log_opportunity(symbol, p_kraken, p_binance, spread, net_spread, analysis):
    with open(LOG_FILE, mode='a', newline='', encoding='utf-8') as f:
        writer = csv.writer(f)
        writer.writerow([time.strftime('%Y-%m-%d %H:%M:%S'), symbol, p_kraken, p_binance, f"{spread:.4f}%", f"{net_spread:.4f}%", analysis])

def log_spread_observation(symbol, price_kraken, price_binance, spread, net_spread, threshold_reached, opportunity_triggered):
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
    }
    with spread_log_lock:
        with open(SPREAD_LOG_FILE, mode='a', newline='', encoding='utf-8') as f:
            writer = csv.DictWriter(f, fieldnames=SPREAD_LOG_HEADERS)
            writer.writerow(row)

def retry_after_seconds(value):
    if not value:
        return 0.0
    try:
        return max(0.0, float(value))
    except ValueError:
        try:
            retry_at = parsedate_to_datetime(value)
            if retry_at.tzinfo is None:
                retry_at = retry_at.replace(tzinfo=timezone.utc)
            return max(0.0, (retry_at - datetime.now(timezone.utc)).total_seconds())
        except (TypeError, ValueError, OverflowError):
            return 0.0

def send_ntfy_notification(title, message):
    global ntfy_blocked_until, ntfy_backoff_seconds

    if not NTFY_TOPIC_URL:
        return False

    with ntfy_lock:
        now = time.monotonic()
        if now < ntfy_blocked_until:
            return False

        try:
            response = requests.post(
                NTFY_TOPIC_URL,
                data=message.encode('utf-8'),
                headers={"Title": title},
                timeout=5
            )
        except requests.RequestException as e:
            ntfy_blocked_until = time.monotonic() + NTFY_GLOBAL_COOLDOWN_SECONDS
            print(f"  ⚠️ Échec de la notification ntfy : {e}")
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
        except requests.RequestException as e:
            ntfy_blocked_until = time.monotonic() + NTFY_GLOBAL_COOLDOWN_SECONDS
            print(f"  ⚠️ Échec de la notification ntfy : {e}")
            return False

        ntfy_blocked_until = time.monotonic() + NTFY_GLOBAL_COOLDOWN_SECONDS
        ntfy_backoff_seconds = NTFY_INITIAL_BACKOFF_SECONDS
        return True

def notify_opportunity(symbol, net_spread):
    return send_ntfy_notification(
        "Pravda-Bot Opportunity",
        f"Spread net : {net_spread:.2f}% sur {symbol}"
    )

def notify_price_movement(exchange_name, symbol, previous_price, current_price, change_pct):
    return send_ntfy_notification(
        "Pravda-Bot Price Movement",
        f"{exchange_name} {symbol}: {previous_price} -> {current_price} ({change_pct:+.4f}%)"
    )

async def handle_price_update(exchange_name, symbol, raw_price):
    global price_updates_count, opportunities_count

    try:
        price = float(raw_price)
        if not math.isfinite(price) or price <= 0:
            raise ValueError("price must be finite and positive")
    except (TypeError, ValueError):
        print(f"  ⚠️ [GARDE-FOU] Prix invalide reçu de {exchange_name} pour {symbol}: {raw_price}")
        return

    previous_price = latest_prices[exchange_name].get(symbol)
    latest_prices[exchange_name][symbol] = price
    price_updates_count += 1

    if previous_price is not None and price != previous_price:
        change_pct = (price - previous_price) / previous_price * 100
        now = time.monotonic()
        if now - last_movement_notification_at.get(symbol, 0) >= PRICE_ALERT_COOLDOWN_SECONDS:
            last_movement_notification_at[symbol] = now
            await asyncio.to_thread(
                notify_price_movement,
                exchange_name,
                symbol,
                previous_price,
                price,
                change_pct
            )

    price_kraken = latest_prices["Kraken"].get(symbol)
    price_binance = latest_prices["Binance"].get(symbol)
    if price_kraken is None or price_binance is None:
        return

    spread = abs(price_kraken - price_binance) / min(price_kraken, price_binance) * 100
    net_spread = spread - ESTIMATED_FEES
    threshold_reached = spread >= MIN_SPREAD_THRESHOLD
    is_profitable = threshold_reached and net_spread > 0
    opportunity_triggered = is_profitable and not last_profitable_state[symbol]

    now = time.monotonic()
    last_logged_at = last_spread_log_at.get(symbol)
    if (
        last_logged_at is None
        or now - last_logged_at >= SPREAD_LOG_INTERVAL_SECONDS
        or opportunity_triggered
    ):
        last_spread_log_at[symbol] = now
        try:
            await asyncio.to_thread(
                log_spread_observation,
                symbol,
                price_kraken,
                price_binance,
                spread,
                net_spread,
                threshold_reached,
                opportunity_triggered,
            )
            spread_summaries[symbol].record(
                spread,
                net_spread,
                threshold_reached,
                opportunity_triggered,
            )
        except OSError as e:
            console.print(f"[bold red]Échec de journalisation du spread : {e}[/bold red]")

    if not threshold_reached:
        last_profitable_state[symbol] = False
        return

    if net_spread <= 0:
        last_profitable_state[symbol] = False
        return

    if last_profitable_state[symbol]:
        return

    last_profitable_state[symbol] = True
    opportunities_count += 1
    console.print(f"[bold green]🎯 Opportunité détectée : {symbol} ({net_spread:.4f}% net estimé)[/bold green]")
    await asyncio.to_thread(notify_opportunity, symbol, net_spread)
    analysis = await asyncio.to_thread(ask_llm_analysis, symbol, price_kraken, price_binance, spread)
    console.print(f"  [green]🤖 LLM: {escape(analysis)}[/green]")
    await asyncio.to_thread(log_opportunity, symbol, price_kraken, price_binance, spread, net_spread, analysis)

async def stream_kraken_tickers():
    async with websockets.connect("wss://ws.kraken.com/v2", ping_interval=20, ping_timeout=20, open_timeout=15) as websocket:
        await websocket.send(json.dumps({
            "method": "subscribe",
            "params": {
                "channel": "ticker",
                "symbol": SYMBOLS,
                "event_trigger": "trades",
                "snapshot": True
            },
            "req_id": 1
        }))

        async for raw_message in websocket:
            message = json.loads(raw_message)
            if message.get("channel") != "ticker":
                continue
            for ticker in message.get("data", []):
                symbol = ticker.get("symbol")
                if symbol in SYMBOLS and ticker.get("last") is not None:
                    await handle_price_update("Kraken", symbol, ticker["last"])

async def stream_binance_tickers():
    streams = "/".join(f"{symbol.replace('/', '').lower()}@ticker" for symbol in SYMBOLS)
    url = f"wss://stream.binance.com:9443/stream?streams={streams}"
    async with websockets.connect(url, ping_interval=20, ping_timeout=20, open_timeout=15) as websocket:
        async for raw_message in websocket:
            message = json.loads(raw_message)
            ticker = message.get("data", message)
            symbol = BINANCE_SYMBOLS.get(ticker.get("s"))
            if symbol is not None and ticker.get("c") is not None:
                await handle_price_update("Binance", symbol, ticker["c"])

async def run_stream_with_reconnect(exchange_name, stream_function):
    while True:
        try:
            await stream_function()
        except asyncio.CancelledError:
            raise
        except Exception as e:
            print(f"  ⚠️ Flux {exchange_name} interrompu : {e}. Reconnexion dans {STREAM_RECONNECT_SECONDS}s.")
            await asyncio.sleep(STREAM_RECONNECT_SECONDS)

async def render_dashboard_periodically():
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
            )
        )

async def monitor_markets():
    print(f"=== Pravda-Bot Temps Réel (Mode: {TRADING_MODE}) ===")
    print(f"Paires surveillées : {', '.join(SYMBOLS)}")
    print(f"Seuil de déclenchement : {MIN_SPREAD_THRESHOLD:g}%")
    print(f"Échantillonnage des spreads : {SPREAD_LOG_INTERVAL_SECONDS:g}s par paire")
    print(f"Tableau de bord Rich : toutes les {DASHBOARD_INTERVAL_SECONDS:g}s")
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
        render_dashboard_periodically(),
    )

try:
    asyncio.run(monitor_markets())
except KeyboardInterrupt:
    print("\n=== Arrêt du bot demandé ===")
    print(f"Mises à jour de prix reçues : {price_updates_count}")
    print(f"Opportunités analysées : {opportunities_count}")
    print("Fermeture propre.")
