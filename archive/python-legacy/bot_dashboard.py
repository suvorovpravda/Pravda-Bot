from dataclasses import dataclass
import time
from typing import Mapping

from rich.table import Table

from market_data_health import assess_market_data


@dataclass
class SpreadSummary:
    samples: int = 0
    gross_total: float = 0.0
    gross_max: float = 0.0
    threshold_hits: int = 0
    positive_net: int = 0
    opportunities: int = 0

    def record(
        self,
        gross_spread: float,
        net_spread: float,
        threshold_reached: bool,
        opportunity_triggered: bool,
    ) -> None:
        self.samples += 1
        self.gross_total += gross_spread
        self.gross_max = max(self.gross_max, gross_spread)
        self.threshold_hits += int(threshold_reached)
        self.positive_net += int(net_spread > 0)
        self.opportunities += int(opportunity_triggered)

    @property
    def gross_average(self):
        return self.gross_total / self.samples if self.samples else None


def build_dashboard(
    symbols: list[str],
    latest_prices: Mapping[str, Mapping[str, float]],
    summaries: Mapping[str, SpreadSummary],
    *,
    threshold: float,
    updates: int,
    opportunities: int,
    last_price_update_at: Mapping[str, Mapping[str, float]] | None = None,
    stale_after_seconds: float = 15,
    max_skew_seconds: float = 3,
    paper_simulator=None,
) -> Table:
    table = Table(title="Pravda-Bot · état du marché")
    table.add_column("Paire", style="yellow")
    table.add_column("Kraken", justify="right")
    table.add_column("Binance", justify="right")
    table.add_column("Spread actuel", justify="right")
    table.add_column("Moy. échantillons", justify="right")
    table.add_column("Max. échantillon", justify="right")
    table.add_column("Seuil atteint", justify="right")
    table.add_column("Net positif", justify="right")
    table.add_column("Flux", justify="center")
    table.add_column("Âge K/B", justify="right")
    table.add_column("Désync.", justify="right")

    now = time.monotonic()
    for symbol in symbols:
        kraken = latest_prices["Kraken"].get(symbol)
        binance = latest_prices["Binance"].get(symbol)
        summary = summaries.get(symbol, SpreadSummary())
        updates_at = last_price_update_at or {"Kraken": {}, "Binance": {}}
        health = assess_market_data(
            updates_at["Kraken"].get(symbol),
            updates_at["Binance"].get(symbol),
            now=now,
            stale_after_seconds=stale_after_seconds,
            max_skew_seconds=max_skew_seconds,
        )
        if kraken is None or binance is None:
            current_spread = "—"
        else:
            current_spread = f"{abs(kraken - binance) / min(kraken, binance) * 100:.4f}%"

        average = f"{summary.gross_average:.4f}%" if summary.samples else "—"
        maximum = f"{summary.gross_max:.4f}%" if summary.samples else "—"
        hit_rate = (
            f"{summary.threshold_hits}/{summary.samples}"
            if summary.samples
            else "—"
        )
        net_rate = (
            f"{summary.positive_net}/{summary.samples}"
            if summary.samples
            else "—"
        )
        table.add_row(
            symbol,
            f"{kraken:,.8g}" if kraken is not None else "—",
            f"{binance:,.8g}" if binance is not None else "—",
            current_spread,
            average,
            maximum,
            hit_rate,
            net_rate,
            health.status,
            (
                f"{health.kraken_age_seconds:.1f}/{health.binance_age_seconds:.1f}s"
                if health.kraken_age_seconds is not None
                and health.binance_age_seconds is not None
                else "—"
            ),
            f"{health.exchange_skew_seconds:.1f}s"
            if health.exchange_skew_seconds is not None
            else "—",
        )

    caption = (
        f"Seuil brut {threshold:g}% · observations {sum(s.samples for s in summaries.values())} "
        f"· mises à jour {updates} · opportunités {opportunities}"
    )
    if paper_simulator is not None:
        caption += (
            f" · paper {paper_simulator.mode} · cash "
            f"{paper_simulator.cash_usdt:.2f} USDT · positions "
            f"{len(paper_simulator.positions)} · P&L réalisé "
            f"{paper_simulator.realized_pnl_usdt:+.2f} USDT"
        )
    table.caption = caption
    return table
