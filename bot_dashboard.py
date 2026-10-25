from dataclasses import dataclass

from rich.table import Table


@dataclass
class SpreadSummary:
    samples: int = 0
    gross_total: float = 0.0
    gross_max: float = 0.0
    threshold_hits: int = 0
    positive_net: int = 0
    opportunities: int = 0

    def record(self, gross_spread, net_spread, threshold_reached, opportunity_triggered):
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
    symbols,
    latest_prices,
    summaries,
    *,
    threshold,
    updates,
    opportunities,
):
    table = Table(title="Pravda-Bot · état du marché")
    table.add_column("Paire", style="yellow")
    table.add_column("Kraken", justify="right")
    table.add_column("Binance", justify="right")
    table.add_column("Spread actuel", justify="right")
    table.add_column("Moy. échantillons", justify="right")
    table.add_column("Max. échantillon", justify="right")
    table.add_column("Seuil atteint", justify="right")
    table.add_column("Net positif", justify="right")

    for symbol in symbols:
        kraken = latest_prices["Kraken"].get(symbol)
        binance = latest_prices["Binance"].get(symbol)
        summary = summaries.get(symbol, SpreadSummary())
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
        )

    table.caption = (
        f"Seuil brut {threshold:g}% · observations {sum(s.samples for s in summaries.values())} "
        f"· mises à jour {updates} · opportunités {opportunities}"
    )
    return table
