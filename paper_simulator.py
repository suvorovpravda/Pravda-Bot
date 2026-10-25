import csv
import math
import threading
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Literal


@dataclass(frozen=True)
class PaperPosition:
    symbol: str
    notional_usdt: float
    entry_signed_spread_percent: float
    opened_at: datetime


class PaperSimulator:
    def __init__(
        self,
        *,
        mode: Literal["fixed", "portfolio"],
        starting_balance_usdt: float,
        position_percent: float,
        fixed_position_usdt: float,
        exit_spread_percent: float,
        max_hold_seconds: float,
        estimated_fees_percent: float,
        log_path: str | Path = "paper_trades.csv",
    ) -> None:
        if mode not in {"fixed", "portfolio"}:
            raise ValueError("paper mode must be 'fixed' or 'portfolio'")
        numeric_settings = {
            "starting balance": starting_balance_usdt,
            "position percentage": position_percent,
            "fixed position": fixed_position_usdt,
            "exit spread": exit_spread_percent,
            "maximum hold": max_hold_seconds,
            "estimated fees": estimated_fees_percent,
        }
        for name, value in numeric_settings.items():
            if not math.isfinite(value) or value < 0:
                raise ValueError(f"{name} must be finite and non-negative")
        if starting_balance_usdt <= 0 or fixed_position_usdt <= 0 or max_hold_seconds <= 0:
            raise ValueError("starting balance, fixed position, and maximum hold must be positive")
        if not 0 < position_percent <= 100:
            raise ValueError("position percentage must be greater than 0 and at most 100")

        self.mode = mode
        self.starting_balance_usdt = starting_balance_usdt
        self.cash_usdt = starting_balance_usdt
        self.position_percent = position_percent
        self.fixed_position_usdt = fixed_position_usdt
        self.exit_spread_percent = exit_spread_percent
        self.max_hold_seconds = max_hold_seconds
        self.estimated_fees_percent = estimated_fees_percent
        self.log_path = Path(log_path)
        self.positions: dict[str, PaperPosition] = {}
        self.realized_pnl_usdt = 0.0
        self.completed_positions = 0
        self._lock = threading.Lock()
        self._ensure_log_header()

    def _ensure_log_header(self) -> None:
        if self.log_path.exists() and self.log_path.stat().st_size > 0:
            return
        with self.log_path.open("w", encoding="utf-8", newline="") as target:
            csv.writer(target).writerow(
                [
                    "Opened_At_UTC",
                    "Closed_At_UTC",
                    "Symbol",
                    "Mode",
                    "Notional_USDT",
                    "Entry_Signed_Spread_Percent",
                    "Exit_Signed_Spread_Percent",
                    "Gross_PnL_USDT",
                    "Estimated_Fees_USDT",
                    "Net_PnL_USDT",
                    "Close_Reason",
                ]
            )

    @staticmethod
    def signed_spread_percent(kraken_price: float, binance_price: float) -> float:
        return (kraken_price - binance_price) / binance_price * 100

    @property
    def open_notional_usdt(self) -> float:
        return sum(position.notional_usdt for position in self.positions.values())

    @property
    def equity_usdt(self) -> float:
        return self.cash_usdt + self.open_notional_usdt

    def _position_size(self) -> float:
        if self.mode == "fixed":
            return min(self.fixed_position_usdt, self.cash_usdt)
        return self.cash_usdt * self.position_percent / 100

    def _log_closed_position(
        self,
        position: PaperPosition,
        signed_spread: float,
        gross_pnl: float,
        fees: float,
        net_pnl: float,
        reason: str,
        now: datetime,
    ) -> None:
        with self.log_path.open("a", encoding="utf-8", newline="") as target:
            writer = csv.writer(target)
            writer.writerow(
                [
                    position.opened_at.isoformat(timespec="milliseconds").replace("+00:00", "Z"),
                    now.isoformat(timespec="milliseconds").replace("+00:00", "Z"),
                    position.symbol,
                    self.mode,
                    f"{position.notional_usdt:.8f}",
                    f"{position.entry_signed_spread_percent:.8f}",
                    f"{signed_spread:.8f}",
                    f"{gross_pnl:.8f}",
                    f"{fees:.8f}",
                    f"{net_pnl:.8f}",
                    reason,
                ]
            )

    def update(
        self,
        symbol: str,
        kraken_price: float,
        binance_price: float,
        *,
        opportunity_confirmed: bool = False,
        now: datetime | None = None,
    ) -> dict[str, str | float] | None:
        if not math.isfinite(kraken_price) or kraken_price <= 0:
            raise ValueError("Kraken price must be finite and positive")
        if not math.isfinite(binance_price) or binance_price <= 0:
            raise ValueError("Binance price must be finite and positive")

        current_time = now or datetime.now(timezone.utc)
        if current_time.tzinfo is None:
            raise ValueError("simulation timestamps must include a timezone")
        current_time = current_time.astimezone(timezone.utc)
        signed_spread = self.signed_spread_percent(kraken_price, binance_price)

        with self._lock:
            position = self.positions.get(symbol)
            if position is not None:
                entry_direction = 1 if position.entry_signed_spread_percent > 0 else -1
                remaining_signed_spread = signed_spread * entry_direction
                age_seconds = (current_time - position.opened_at).total_seconds()
                close_reason = None
                if remaining_signed_spread <= self.exit_spread_percent:
                    close_reason = "spread_target"
                elif age_seconds >= self.max_hold_seconds:
                    close_reason = "max_hold"
                if close_reason is None:
                    return None

                gross_pnl = (
                    (abs(position.entry_signed_spread_percent) - remaining_signed_spread)
                    * position.notional_usdt
                    / 100
                )
                fees = position.notional_usdt * self.estimated_fees_percent / 100
                net_pnl = gross_pnl - fees
                self._log_closed_position(
                    position, signed_spread, gross_pnl, fees, net_pnl, close_reason, current_time
                )
                self.cash_usdt += position.notional_usdt + net_pnl
                self.realized_pnl_usdt += net_pnl
                self.completed_positions += 1
                del self.positions[symbol]
                return {
                    "action": "closed",
                    "symbol": symbol,
                    "net_pnl_usdt": net_pnl,
                    "reason": close_reason,
                }

            if not opportunity_confirmed:
                return None

            notional = self._position_size()
            if notional <= 0:
                return None
            self.cash_usdt -= notional
            self.positions[symbol] = PaperPosition(
                symbol=symbol,
                notional_usdt=notional,
                entry_signed_spread_percent=signed_spread,
                opened_at=current_time,
            )
            return {
                "action": "opened",
                "symbol": symbol,
                "notional_usdt": notional,
                "entry_signed_spread_percent": signed_spread,
            }
