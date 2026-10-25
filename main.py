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

load_dotenv()

console = Console()

LM_URL = os.getenv("LM_STUDIO_API_BASE", "http://192.168.1.146:1234/v1")
TRADING_MODE = os.getenv("TRADING_MODE", "paper")
MODEL_NAME = os.getenv("LM_MODEL_NAME", "qwen/qwen3.5-9b")
MIN_SPREAD_THRESHOLD = 0.05  # Modifie cette valeur selon tes besoins
ESTIMATED_FEES = 0.36
PRICE_ALERT_COOLDOWN_SECONDS = max(0.0, float(os.getenv("PRICE_ALERT_COOLDOWN_SECONDS", "5")))
NTFY_GLOBAL_COOLDOWN_SECONDS = max(1.0, float(os.getenv("NTFY_GLOBAL_COOLDOWN_SECONDS", "10")))
NTFY_INITIAL_BACKOFF_SECONDS = 60.0
NTFY_MAX_BACKOFF_SECONDS = 900.0
STREAM_RECONNECT_SECONDS = 5

SYMBOLS = ['BTC/USDT', 'ETH/USDT', 'SOL/USDT']
BINANCE_SYMBOLS = {symbol.replace('/', ''): symbol for symbol in SYMBOLS}
LOG_FILE = "arbitrage_log.csv"
LOG_HEADERS = ["Timestamp", "Symbol", "Kraken_Price", "Binance_Price", "Spread_Percent", "Net_Spread_Estimated", "LLM_Analysis"]
NTFY_TOPIC_URL = os.getenv("NTFY_TOPIC_URL", "").strip()

if not os.path.exists(LOG_FILE):
    with open(LOG_FILE, mode='w', newline='', encoding='utf-8') as f:
        writer = csv.writer(f)
        writer.writerow(LOG_HEADERS)

latest_prices = {"Kraken": {}, "Binance": {}}
last_movement_notification_at = {}
last_profitable_state = {symbol: False for symbol in SYMBOLS}
price_updates_count = 0
opportunities_count = 0
ntfy_lock = threading.Lock()
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
        console.print(
            f"[bold cyan][Pravda-Bot][/bold cyan] [MOUVEMENT] "
            f"{exchange_name} {symbol}: {previous_price} -> {price} "
            f"({change_pct:+.4f}%)"
        )

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
    console.print(
        f"[bold cyan][Pravda-Bot][/bold cyan] Analyse du marché en cours... "
        f"Paire: [yellow]{symbol}[/yellow]"
    )
    console.print(
        f"  -> Kraken : [green]{price_kraken}[/green] | "
        f"Binance : [green]{price_binance}[/green]"
    )
    console.print(
        f"  -> Écart brut : [magenta]{spread:.4f}%[/magenta] "
        f"(Seuil : {MIN_SPREAD_THRESHOLD}%)"
    )

    if spread < MIN_SPREAD_THRESHOLD:
        last_profitable_state[symbol] = False
        console.print("[dim]💤 Écart trop faible, on surveille le prochain tick...[/dim]")
        return

    if net_spread <= 0:
        last_profitable_state[symbol] = False
        console.print(
            f"  [yellow]🛡️ Opportunité écartée sur {symbol} : "
            f"spread net après frais négatif ({net_spread:.4f}%).[/yellow]"
        )
        return

    if last_profitable_state[symbol]:
        return

    last_profitable_state[symbol] = True
    opportunities_count += 1
    console.print(
        "[bold green]🎯 Opportunité détectée ! "
        "Transmission à l'IA pour analyse...[/bold green]"
    )
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

async def monitor_markets():
    print(f"=== Pravda-Bot Temps Réel (Mode: {TRADING_MODE}) ===")
    print(f"Paires surveillées : {', '.join(SYMBOLS)}")
    print(
        "Notifications ntfy : configurées"
        if NTFY_TOPIC_URL
        else "Notifications ntfy : désactivées (NTFY_TOPIC_URL absent)"
    )
    print(f"Délai anti-rafale par paire : {PRICE_ALERT_COOLDOWN_SECONDS:g}s\n")
    print(f"Délai global ntfy : {NTFY_GLOBAL_COOLDOWN_SECONDS:g}s; backoff initial après 429 : {NTFY_INITIAL_BACKOFF_SECONDS:g}s\n")
    await asyncio.gather(
        run_stream_with_reconnect("Kraken", stream_kraken_tickers),
        run_stream_with_reconnect("Binance", stream_binance_tickers)
    )

try:
    asyncio.run(monitor_markets())
except KeyboardInterrupt:
    print("\n=== Arrêt du bot demandé ===")
    print(f"Mises à jour de prix reçues : {price_updates_count}")
    print(f"Opportunités analysées : {opportunities_count}")
    print("Fermeture propre.")
