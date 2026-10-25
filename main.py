import os
import time
import csv
import math
import requests
import ccxt
from dotenv import load_dotenv

load_dotenv()

LM_URL = os.getenv("LM_STUDIO_API_BASE", "http://192.168.1.146:1234/v1")
TRADING_MODE = os.getenv("TRADING_MODE", "paper")
MODEL_NAME = os.getenv("LM_MODEL_NAME", "qwen-local")
INTERVAL_SECONDS = 10
SPREAD_THRESHOLD = 0.05
ESTIMATED_FEES = 0.36

SYMBOLS = ['BTC/USDT', 'ETH/USDT', 'SOL/USDT']
LOG_FILE = "arbitrage_log.csv"
LOG_HEADERS = ["Timestamp", "Symbol", "Kraken_Price", "Binance_Price", "Spread_Percent", "Net_Spread_Estimated", "LLM_Analysis"]

if not os.path.exists(LOG_FILE):
    with open(LOG_FILE, mode='w', newline='', encoding='utf-8') as f:
        writer = csv.writer(f)
        writer.writerow(LOG_HEADERS)

kraken = ccxt.kraken()
binance = ccxt.binance()

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

print(f"=== Pravda-Bot Sécurisé (Mode: {TRADING_MODE}) ===")
print(f"Paires surveillées : {', '.join(SYMBOLS)}")
print(f"Journalisation active dans : {LOG_FILE}\n")

checks_count = 0
opportunities_count = 0

try:
    while True:
        checks_count += 1
        for symbol in SYMBOLS:
            try:
                ticker_k = kraken.fetch_ticker(symbol)
                ticker_b = binance.fetch_ticker(symbol)
                
                p_kraken = ticker_k.get('last')
                p_binance = ticker_b.get('last')

                try:
                    prices_valid = all(
                        math.isfinite(price) and price > 0
                        for price in (p_kraken, p_binance)
                    )
                except (TypeError, ValueError):
                    prices_valid = False

                if not prices_valid:
                    print(f"  ⚠️ [GARDE-FOU] Prix invalides reçus pour {symbol} (Kraken: {p_kraken}, Binance: {p_binance}). Ignoré.")
                    continue
                
                spread = abs(p_kraken - p_binance) / min(p_kraken, p_binance) * 100
                net_spread = spread - ESTIMATED_FEES
                
                timestamp = time.strftime('%H:%M:%S')
                print(f"[{timestamp}] {symbol:<8} | Kraken: {p_kraken:<9} | Binance: {p_binance:<9} | Écart Brut: {spread:.4f}%")
                
                if spread >= SPREAD_THRESHOLD:
                    if net_spread <= 0:
                        print(f"  🛡️ [GARDE-FOU] Opportunité écartée sur {symbol} : Spread brut ({spread:.4f}%) inférieur aux frais estimés ({ESTIMATED_FEES}%).")
                        continue

                    opportunities_count += 1
                    print("\a")
                    print(f"\033[91m  ⚡ [ALERTE RENTABLE] Opportunité valide sur {symbol} (Net: {net_spread:.4f}%) !\033[0m")
                    print("  🤖 Analyse LLM en cours...")
                    analysis = ask_llm_analysis(symbol, p_kraken, p_binance, spread)
                    print(f"  \033[92m🤖 LLM: {analysis}\033[0m")
                    log_opportunity(symbol, p_kraken, p_binance, spread, net_spread, analysis)
                    
            except Exception as e:
                print(f"  ⚠️ Erreur sur {symbol} : {e}")

        print("-" * 60)
        time.sleep(INTERVAL_SECONDS)

except KeyboardInterrupt:
    print("\n=== Arrêt du bot demandé ===")
    print(f"Total cycles exécutés : {checks_count}")
    print(f"Opportunités analysées : {opportunities_count}")
    print("Fermeture propre.")
