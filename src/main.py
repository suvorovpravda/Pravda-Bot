"""Main - Bot d'arbitrage Pravda avec validation multi-couches."""

from __future__ import annotations

import csv
import logging
import os
import time
from typing import Any

# Import des modules core
from src.exchange_client import ExchangeClient
from src.risk_manager import RiskManager
from src.lm_studio_client import LMStudioClient

logger = logging.getLogger("Pravda_Bot")


def load_env_vars() -> dict[str, str]:
    """Charger les variables d'environnement depuis .env."""
    from dotenv import load_dotenv
    
    load_dotenv()
    
    return {
        "LM_STUDIO_API_BASE": os.getenv("LM_STUDIO_API_BASE", "http://localhost:1234/v1"),
        "TRADING_MODE": os.getenv("TRADING_MODE", "paper"),
        "LM_MODEL_NAME": os.getenv("LM_MODEL_NAME", "qwen3.5-9b"),
    }


def ask_llm_analysis(
    symbol: str, 
    price_kraken: float, 
    price_binance: float, 
    spread: float
) -> str:
    """Demander une analyse qualitative à l'IA locale.

    Args:
        symbol: Symbole de la paire (ex: "ETH/USDT")
        price_kraken: Prix sur Kraken
        price_binance: Prix sur Binance
        spread: Spread brut en %

    Returns:
        Analyse textuelle du modèle ou message d'erreur
    """
    env_vars = load_env_vars()
    
    prompt = (
        f"Analyse le spread d'arbitrage pour {symbol}. "
        f"Kraken: {price_kraken:.2f}, Binance: {price_binance:.2f}, "
        f"Écart brut: {spread:.2f}%. "
        f"Frais estimés totaux: ~0.36%. "
        f"Est-ce une opportunité valide après frais d'échange ? "
        f"Réponds brièvement en français."
    )

    try:
        lm_client = LMStudioClient()
        res = lm_client.query_local_model(prompt)
        
        # Nettoyer les retours à la ligne pour le log CSV
        return res.replace('\n', ' ').strip()
    
    except Exception as e:
        logger.error("Erreur LLM : %s", e)
        return f"Erreur LLM : {e}"


def log_opportunity(
    symbol: str, 
    p_kraken: float, 
    p_binance: float, 
    spread: float, 
    net_spread: float, 
    analysis: str
) -> None:
    """Enregistrer l'opportunité dans le journal CSV."""
    LOG_FILE = "arbitrage_log.csv"
    
    if not os.path.exists(LOG_FILE):
        with open(LOG_FILE, mode='w', newline='', encoding='utf-8') as f:
            writer = csv.writer(f)
            writer.writerow([
                "Timestamp", 
                "Symbol", 
                "Kraken_Price", 
                "Binance_Price", 
                "Spread_Percent", 
                "Net_Spread_Estimated", 
                "LLM_Analysis"
            ])

    with open(LOG_FILE, mode='a', newline='', encoding='utf-8') as f:
        writer = csv.writer(f)
        writer.writerow([
            time.strftime('%Y-%m-%d %H:%M:%S'), 
            symbol, 
            p_kraken, 
            p_binance, 
            f"{spread:.4f}%", 
            f"{net_spread:.4f}%", 
            analysis
        ])


def main() -> None:
    """Boucle principale du bot d'arbitrage."""
    
    # Configuration
    env_vars = load_env_vars()
    SYMBOLS = ['BTC/USDT', 'ETH/USDT', 'SOL/USDT']
    SPREAD_THRESHOLD = 0.05  # 0.05% = seuil d'intérêt minimum
    ESTIMATED_FEES = 0.36     # Frais totaux estimés (~0.26% Kraken + ~0.10% Binance)
    
    # Initialisation des clients
    exchange_client = ExchangeClient()
    risk_manager = RiskManager(
        max_drawdown_daily=500.0,      # $500 perte quotidienne max
        max_position_size_usd=100.0,   # $100 taille position max (paper trading)
        min_net_margin_pct=ESTIMATED_FEES,  # Marge nette min = frais estimés
    )
    
    logger.info(f"=== Pravda-Bot Sécurisé (Mode: {env_vars['TRADING_MODE']}) ===")
    logger.info(f"Paires surveillées : {', '.join(SYMBOLS)}")
    logger.info(f"Frais estimés : {ESTIMATED_FEES}%")
    logger.info(f"Seuil spread intérêt : {SPREAD_THRESHOLD}%")
    
    # Initialisation du journal CSV
    log_opportunity("INIT", 0, 0, 0, 0, "")

    checks_count = 0
    opportunities_count = 0
    
    try:
        while True:
            checks_count += 1
            
            for symbol in SYMBOLS:
                try:
                    # Récupération des tickers
                    ticker_k = exchange_client._get_exchange("kraken").fetch_ticker(symbol)
                    ticker_b = exchange_client._get_exchange("binance").fetch_ticker(symbol)

                    p_kraken = ticker_k.get('last')
                    p_binance = ticker_b.get('last')

                    # GARDE-FOU 1: Vérification prix valides et non nuls
                    if not p_kraken or not p_binance or p_kraken <= 0 or p_binance <= 0:
                        print(f"  ⚠️ [GARDE-FOU] Prix invalides reçus pour {symbol}. Ignoré.")
                        continue

                    # Calcul du spread brut
                    spread = abs(p_kraken - p_binance) / min(p_kraken, p_binance) * 100
                    net_spread = spread - ESTIMATED_FEES

                    timestamp = time.strftime('%H:%M:%S')
                    print(f"[{timestamp}] {symbol:<8} | Kraken: {p_kraken:<9.4f} | Binance: {p_binance:<9.4f} | Écart Brut: {spread:.4f}%")

                    # Vérification spread intéressant
                    if spread >= SPREAD_THRESHOLD:
                        # GARDE-FOU 2: Validation du spread net (doit couvrir les frais)
                        if net_spread <= 0:
                            print(f"  🛡️ [GARDE-FOU] Opportunité non rentable sur {symbol} "
                                  f"(Net: {net_spread:.4f}%). Dropped.")
                            continue

                        opportunities_count += 1
                        
                        # Signal sonore et visuel pour opportunité rentable
                        print("\a")
                        print(f"\033[91m  ⚡ [ALERTE RENTABLE] Opportunité valide sur {symbol} "
                              f"(Net: {net_spread:.4f}%) !\033[0m")
                        
                        # Analyse qualitative LLM locale (Qwen RTX 3090)
                        print("  🤖 Analyse LLM en cours...")
                        analysis = ask_llm_analysis(symbol, p_kraken, p_binance, spread)
                        print(f"  \033[92m🤖 LLM: {analysis}\033[0m")
                        
                        # Log de l'opportunité
                        log_opportunity(symbol, p_kraken, p_binance, spread, net_spread, analysis)

                except Exception as e:
                    print(f"  ⚠️ Erreur sur {symbol} : {e}")

            print("-" * 60)
            time.sleep(10)  # Éviter de saturer les API (10 secondes entre chaque cycle)

    except KeyboardInterrupt:
        print(f"\n=== Arrêt du bot demandé ===")
        print(f"Total cycles exécutés : {checks_count}")
        print(f"Opportunités analysées : {opportunities_count}")
        print("Fermeture propre.")


if __name__ == "__main__":
    main()
