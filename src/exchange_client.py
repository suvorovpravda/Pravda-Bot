"""Exchange Client - Communication avec les bourses via ccxt."""

from __future__ import annotations

import logging
import os
from typing import Any

logger = logging.getLogger("Pravda_Exchange")


class ExchangeClient:
    """Client pour récupérer les prix en temps réel via ccxt.

    Se connecte aux exchanges (Kraken, Binance) et récupère :
    - Les carnets d'ordres (orderbooks)
    - Les tickers de prix
    - Les spreads Ask/Bid
    """

    def __init__(self):
        """Initialiser le client avec chargement des clés API depuis .env."""
        self._exchanges: dict[str, Any] = {}
        self._api_keys: dict[str, str] = {
            "kraken": os.getenv("KRAKEN_API_KEY", ""),
            "binance": os.getenv("BINANCE_API_KEY", ""),
        }
        self._api_secrets: dict[str, str] = {
            "kraken": os.getenv("KRAKEN_SECRET_KEY", ""),
            "binance": os.getenv("BINANCE_SECRET_KEY", ""),
        }

    def _get_exchange(self, exchange_id: str) -> Any:
        """Obtenir ou créer une instance d'échange ccxt."""
        if exchange_id not in self._exchanges:
            try:
                import ccxt
                
                if exchange_id == "kraken":
                    exchange = ccxt.kraken({
                        'apiKey': self._api_keys.get("kraken"),
                        'secret': self._api_secrets.get("kraken"),
                        'enableRateLimit': True,
                        'timeout': 30000,
                    })
                elif exchange_id == "binance":
                    exchange = ccxt.binance({
                        'apiKey': self._api_keys.get("binance"),
                        'secret': self._api_secrets.get("binance"),
                        'enableRateLimit': True,
                        'timeout': 30000,
                    })
                else:
                    raise ValueError(f"Exchange inconnu : {exchange_id}")
                
                # Charger les marchés (optionnel pour le mode public)
                try:
                    import asyncio
                    asyncio.get_event_loop().run_until_complete(
                        exchange.load_markets()
                    )
                except Exception as load_error:
                    logger.warning("Chargement des marchés ignoré (mode public): %s", load_error)
                
                self._exchanges[exchange_id] = exchange
                
            except ImportError:
                raise ImportError(
                    "La bibliothèque ccxt n'est pas installée. "
                    "Installez-la avec : pip install ccxt"
                )
        
        return self._exchanges[exchange_id]

    def get_ticker_spread(
        self, 
        symbol: str, 
        exchange_buy: str = "kraken", 
        exchange_sell: str = "binance"
    ) -> dict[str, Any]:
        """Récupérer le spread Ask/Bid entre deux exchanges.

        Args:
            symbol: Symbole de la paire (ex: "ETH/USDT")
            exchange_buy: Exchange d'achat (ex: "kraken")
            exchange_sell: Exchange de vente (ex: "binance")

        Returns:
            Dictionnaire avec :
            - ask_price: Prix Ask sur l'exchange d'achat
            - bid_price: Prix Bid sur l'exchange de vente
            - spread_brut_pct: Spread brut en pourcentage
            - fees_estimee_buy: Frais estimés exchange achat
            - fees_estimee_sell: Frais estimés exchange vente

        Raises:
            TimeoutError: Si l'appel API dépasse le timeout
            ConnectionError: Si l'API de l'échange est indisponible
        """
        try:
            ticker_buy = self._get_exchange(exchange_buy).fetch_ticker(symbol)
            ticker_sell = self._get_exchange(exchange_sell).fetch_ticker(symbol)

            # Récupération des prix
            ask_price = float(ticker_buy.get('ask', 0))
            bid_price = float(ticker_sell.get('bid', 0))

            # Calcul du spread brut en pourcentage
            if bid_price > 0:
                spread_brut_pct = abs(ask_price - bid_price) / bid_price * 100
            else:
                spread_brut_pct = 0.0

            # Frais estimés (typiques par exchange)
            fees_estimee_buy = 0.26  # Kraken ~0.26% spot
            fees_estimee_sell = 0.10  # Binance ~0.10% spot

            return {
                "symbol": symbol,
                "exchange_buy": exchange_buy,
                "exchange_sell": exchange_sell,
                "ask_price": ask_price,
                "bid_price": bid_price,
                "spread_brut_pct": spread_brut_pct,
                "fees_estimee_buy": fees_estimee_buy,
                "fees_estimee_sell": fees_estimee_sell,
            }

        except Exception as error:
            logger.error("Erreur lors de la récupération du ticker %s: %s", symbol, error)
            raise TimeoutError(f"Timeout ou erreur API pour {symbol}: {error}")

    def get_orderbook(
        self, 
        exchange_id: str, 
        symbol: str, 
        limit: int = 10
    ) -> dict[str, Any]:
        """Récupérer le carnet d'ordres (orderbook).

        Args:
            exchange_id: Identifiant de l'échange ("kraken" ou "binance")
            symbol: Symbole de la paire
            limit: Nombre de niveaux d'ordres à récupérer

        Returns:
            Dictionnaire avec ask/bid orders et spreads
        """
        try:
            orderbook = self._get_exchange(exchange_id).fetch_order_book(symbol, limit=limit)
            
            return {
                "asks": orderbook.get('asks', []),
                "bids": orderbook.get('bids', []),
                "symbol": symbol,
                "exchange": exchange_id,
            }

        except Exception as error:
            logger.error("Erreur lors de la récupération du carnet d'ordres %s: %s", symbol, error)
            raise ConnectionError(f"Indisponible pour {symbol}: {error}")
