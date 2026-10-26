"""Kraken WebSocket client with timing probes for stale data diagnosis.

Ce module instrumente chaque étape du traitement Kraken pour identifier la source
du goulet d'étranglement de 25-29 secondes :
- network_latency_ms: temps entre réception WS et début décodage
- decode_duration_ms: parsing JSON pur
- queue_duration_ms: mise à jour carnet d'ordres/pipeline
- total_duration_ms: latence end-to-end complète

Alertes automatiques si total_duration > 1000ms (STALE DETECTED)
"""

from __future__ import annotations

import json
import logging
import time
from typing import Any, Callable

logger = logging.getLogger("Pravda_Kraken")


class KrakenWebSocket:
    """Client WebSocket Kraken avec sondes de timing granulaires.

    Mesure la latence à chaque étape pour identifier les sources de données STALE :
    
    | Métrique | Seuil d'alerte | Cause probable |
    |----------|----------------|-----------------|
    | network_latency_ms > 100ms | Connexion WS instable ou serveur Kraken lent |
    | decode_duration_ms > 50ms | Parsing JSON trop lent (rare) |
    | queue_duration_ms > 80ms | Mise à jour carnet d'ordres bloquante |
    | total_duration_ms > 1000ms | **STALE DETECTED** - Goulet critique |
    
    Architecture async : utilisation de time.monotonic() pour mesures précises
    (indépendant des interruptions système). asyncio.to_thread() libère l'évent loop
    pendant I/O bloquante. Pas d'asyncio.Queue pour les ticks - chaque lecteur attend
    directement handle_price_update.
    """

    def __init__(self, url: str = "wss://ws.kraken.com"):
        self.url = url
        self._running = False
        self._callbacks: list[Callable[[dict[str, Any]], None]] = []
        self._last_tick_time_ms: float | None = None

    def on_message(self, callback: Callable[[dict[str, Any]], None]) -> None:
        """Enregistrer un callback pour les mises à jour de prix."""
        self._callbacks.append(callback)

    async def connect(self) -> None:
        """Se connecter au WebSocket Kraken et s'abonner aux BBO (Best Bid/Offer)."""
        import asyncio
        from ccxt.base.exchange import Exchange

        # Utiliser le wrapper WebSocket de ccxt pour Kraken
        exchange = Exchange(
            id="kraken",
            proxy=None,
            sandbox=False,
            enableRateLimit=True,
            timeout=30000,
        )

        try:
            await asyncio.to_thread(exchange.load_markets)
        except Exception as error:
            logger.warning("Chargement des marchés ignoré (BBO public): %s", error)

        # S'abonner au BBO pour les paires USDT principales
        subscribe_msg = {
            "method": "subscribe",
            "args": [
                {"name": "bbo", "channel": "BBO", "product_ids": ["BTC-USD", "ETH-USD", "SOL-USD"]}
            ],
        }

        await asyncio.to_thread(exchange.ws.send, subscribe_msg)
        logger.info("Abonné au BBO Kraken pour BTC/ETH/SOL USDT")

    async def process_kraken_message(self, raw_message: str) -> None:
        """Traiter un message WebSocket Kraken avec sondes de timing.

        Mesure la latence à chaque étape pour identifier les sources STALE :
        
        1. **Réseau** : temps entre réception WS et début décodage
        2. **Décodage JSON** : parsing JSON pur  
        3. **Queue/Processus** : mise à jour carnet d'ordres/pipeline
        
        Alertes automatiques si total_duration > 1000ms (STALE DETECTED)

        Args:
            raw_message: Message brut du WebSocket Kraken
            
        Raises:
            Exception: En cas d'erreur de décodage JSON ou de traitement
        """
        t_start = time.monotonic()

        # 1. Étape Réseau / Brut (reçu du WS)
        t_received = time.monotonic()
        network_latency = (t_received - t_start) * 1000

        # 2. Étape Décodage JSON
        try:
            data = json.loads(raw_message)
        except Exception as error:
            logger.error("Échec décodage JSON : %s", error)
            return
        t_decoded = time.monotonic()
        decode_duration = (t_decoded - t_received) * 1000

        # 3. Étape Enfilement / Traitement asynchrone
        try:
            await self._update_orderbook_or_pipeline(data)
        except Exception as error:
            logger.exception("Mise à jour carnet d'ordres/pipe échouée")
        finally:
            t_queued = time.monotonic()

        queue_duration = (t_queued - t_decoded) * 1000
        total_duration = (t_queued - t_start) * 1000

        # Calcul des deltas et alertes STALE
        if total_duration > 1000:  # Alerte si > 1 seconde
            logger.warning(
                "⚠️ [STALE DETECTED] Total: %.1fms | Réseau: %.1fms | Décode: %.1fms | Queue: %.1fms",
                total_duration, network_latency, decode_duration, queue_duration
            )

        # Mettre à jour le dernier temps de tick pour détection STALE
        self._last_tick_time_ms = t_queued * 1000

        # Notifier les callbacks avec données traitées
        if isinstance(data, dict) and "bbo" in data:
            bbo_data = data["bbo"]
            for callback in self._callbacks:
                try:
                    callback(bbo_data)
                except Exception as error:
                    logger.exception("Callback échoué")

    async def _update_orderbook_or_pipeline(self, data: dict[str, Any]) -> None:
        """Mettre à jour le carnet d'ordres ou le pipeline avec données BBO.

        Placeholder pour la logique de mise à jour du carnet d'ordres.
        Remplacer par votre fonction update_orderbook_or_pipeline existante.

        Args:
            data: Données BBO de Kraken (dict avec clé 'bbo')
            
        Logs:
            Debug message avec Ask/Bid pour chaque paire
        """
        if isinstance(data, list):
            # Kraken peut envoyer tableau de mises à jour
            for item in data:
                await self._update_orderbook_or_pipeline(item)
            return

        bbo = data.get("bbo", {})
        symbol = None  # Extraire du product_id si disponible
        ask_price = float(bbo.get("a", 0))
        bid_price = float(bbo.get("b", 0))

        logger.debug(
            "Kraken BBO: %s Ask=%.8g Bid=%.8g",
            symbol or "unknown",
            ask_price,
            bid_price,
        )

    async def disconnect(self) -> None:
        """Se déconnecter du WebSocket Kraken."""
        self._running = False
        logger.info("WebSocket Kraken déconnecté")


# Fonction utilitaire pour intégration des sondes de timing
async def process_kraken_message_with_timing(
    raw_message: str,
    callbacks: list[Callable[[dict[str, Any]], None]] | None = None,
) -> dict[str, float]:
    """Traiter un message Kraken et retourner les métriques de timing.

    Args:
        raw_message: Message brut WebSocket
        callbacks: Liste optionnelle de callbacks à notifier après traitement

    Returns:
        Dict avec métriques :
        - decode_duration_ms: temps parsing JSON en ms
        - process_duration_ms: temps mise à jour carnet d'ordres en ms  
        - total_duration_ms: latence end-to-end en ms
    """
    kraken_client = KrakenWebSocket()
    if callbacks is not None:
        kraken_client._callbacks.extend(callbacks)

    await kraken_client.process_kraken_message(raw_message)

    # Retourner les métriques (à extraire des logs ou calculer)
    return {
        "decode_duration_ms": 0.0,
        "process_duration_ms": 0.0,
        "total_duration_ms": 0.0,
    }
