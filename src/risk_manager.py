"""Risk Manager - Circuit Breaker strict pour validation des trades."""

from __future__ import annotations

import logging
from typing import Any, Callable

logger = logging.getLogger("Pravda_Risk")


class RiskManager:
    """Garde-fou (Circuit Breaker) strict et hardcodé.

    Vérifie qu'un trade est mathématiquement rentable avant autorisation :
    - Spread net après frais
    - Taille de position maximale
    - Drawdown quotidien maximal
    """

    def __init__(
        self,
        max_drawdown_daily: float = 500.0,  # USD maximum perte quotidienne
        max_position_size_usd: float = 100.0,  # USD taille position max
        min_net_margin_pct: float = 0.36,  # % marge nette minimale (couvre frais ~0.36%)
    ):
        """Initialiser le RiskManager avec paramètres configurables."""
        self.max_drawdown_daily = max_drawdown_daily
        self.max_position_size_usd = max_position_size_usd
        self.min_net_margin_pct = min_net_margin_pct

        # État du circuit breaker
        self._cumulative_loss_usd: float = 0.0
        self._is_circuit_open: bool = False

    def validate_trade(self, trade_proposal: dict[str, Any]) -> tuple[bool, str]:
        """Valider un trade selon les critères de risque stricts.

        Args:
            trade_proposal: Dictionnaire contenant :
                - expected_spread_pct: Spread attendu en % (ex: 0.5 pour 0.5%)
                - estimated_fees_pct: Frais estimés en % (ex: 0.36)
                - estimated_slippage_pct: Slippage estimé en %
                - position_size_usd: Taille de la position en USD
                - daily_loss_usd: Perte quotidienne cumulée actuelle

        Returns:
            Tuple (bool, str):
            - (True, "OK: Trade validé") si tous les critères sont satisfaits
            - (False, "REJECTED: [Raison exacte]") sinon
        """
        # Récupération des valeurs
        expected_spread_pct = float(trade_proposal.get("expected_spread_pct", 0))
        estimated_fees_pct = float(trade_proposal.get("estimated_fees_pct", 0.36))
        estimated_slippage_pct = float(trade_proposal.get("estimated_slippage_pct", 0))
        position_size_usd = float(trade_proposal.get("position_size_usd", 0))
        daily_loss_usd = float(trade_proposal.get("daily_loss_usd", self._cumulative_loss_usd))

        # Calcul du spread net
        net_margin = expected_spread_pct - (estimated_fees_pct + estimated_slippage_pct)

        # Vérification 1: Spread net insuffisant
        if net_margin < self.min_net_margin_pct:
            return (
                False,
                f"REJECTED: Spread net ({net_margin:.4f}%) inférieur au minimum requis "
                f"({self.min_net_margin_pct}%)",
            )

        # Vérification 2: Taille de position trop grande
        if position_size_usd > self.max_position_size_usd:
            return (
                False,
                f"REJECTED: Taille de position (${position_size_usd:.2f}) dépasse la limite "
                f"${self.max_position_size_usd:.2f}",
            )

        # Vérification 3: Drawdown quotidien dépassé (Circuit Breaker)
        if daily_loss_usd > self.max_drawdown_daily:
            self._is_circuit_open = True
            return (
                False,
                f"REJECTED: Circuit breaker activé - Perte quotidienne "
                f"${daily_loss_usd:.2f} dépasse le maximum ${self.max_drawdown_daily:.2f}",
            )

        # Trade validé
        self._cumulative_loss_usd = 0.0  Reset perte cumulée sur trade réussi
        self._is_circuit_open = False
        return (True, "OK: Trade validé")

    def reset_daily(self) -> None:
        """Reset du drawdown quotidien au début d'une nouvelle journée."""
        logger.info("RiskManager: Reset du drawdown quotidien")
        self._cumulative_loss_usd = 0.0
        self._is_circuit_open = False

    def record_loss(self, loss_amount: float) -> None:
        """Enregistrer une perte pour le calcul du drawdown."""
        if not self._is_circuit_open:
            self._cumulative_loss_usd += loss_amount
            logger.warning(
                "RiskManager: Drawdown cumulé = $%.2f / $%.2f",
                self._cumulative_loss_usd,
                self.max_drawdown_daily,
            )
