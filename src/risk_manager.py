"""Experimental paper-risk utility; not a live execution engine."""
from __future__ import annotations
import math


class RiskManager:
    def __init__(self, max_drawdown_daily=500.0, max_position_size_usd=100.0, min_net_margin_pct=0.36):
        for value in (max_drawdown_daily, max_position_size_usd, min_net_margin_pct):
            if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or value < 0:
                raise ValueError("Risk limits must be finite and non-negative")
        if max_drawdown_daily == 0 or max_position_size_usd == 0:
            raise ValueError("Drawdown and position limits must be positive")
        self.max_drawdown_daily = max_drawdown_daily
        self.max_position_size_usd = max_position_size_usd
        self.min_net_margin_pct = min_net_margin_pct
        self._cumulative_loss_usd = 0.0
        self._is_circuit_open = False

    def validate_trade(self, trade_proposal):
        if self._is_circuit_open:
            return False, "REJECTED: Circuit breaker active"
        try:
            names = ("expected_spread_pct", "estimated_fees_pct", "estimated_slippage_pct", "position_size_usd", "daily_loss_usd")
            defaults = (0, 0.36, 0, 0, self._cumulative_loss_usd)
            raw = [trade_proposal.get(name, default) for name, default in zip(names, defaults)]
            if any(isinstance(value, bool) for value in raw):
                raise ValueError
            spread, fees, slippage, size, loss = map(float, raw)
            if not all(math.isfinite(value) for value in (spread, fees, slippage, size, loss)):
                raise ValueError
            if min(fees, slippage, loss) < 0 or size <= 0:
                raise ValueError
        except (ValueError, TypeError, OverflowError, AttributeError):
            return False, "REJECTED: Invalid risk inputs"
        if max(loss, self._cumulative_loss_usd) >= self.max_drawdown_daily:
            self._is_circuit_open = True
            return False, "REJECTED: Daily drawdown limit"
        if size > self.max_position_size_usd:
            return False, "REJECTED: Position size limit"
        if spread - fees - slippage < self.min_net_margin_pct:
            return False, "REJECTED: Insufficient net margin"
        return True, "OK: Paper proposal validated"

    def record_loss(self, loss_amount):
        if isinstance(loss_amount, bool) or not isinstance(loss_amount, (int, float)) or not math.isfinite(loss_amount) or loss_amount < 0:
            raise ValueError("Loss must be finite and non-negative")
        self._cumulative_loss_usd += loss_amount
        if self._cumulative_loss_usd >= self.max_drawdown_daily:
            self._is_circuit_open = True

    def reset_daily(self):
        self._cumulative_loss_usd = 0.0
        self._is_circuit_open = False
