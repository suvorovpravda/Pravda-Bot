import argparse
import csv
import math
import statistics
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

from rich.console import Console
from rich.table import Table


REQUIRED_COLUMNS = {
    "Timestamp_UTC",
    "Symbol",
    "Gross_Spread_Percent",
    "Estimated_Net_Spread_Percent",
    "Minimum_Trigger_Threshold_Percent",
    "Threshold_Reached",
    "Net_Spread_Positive",
    "Opportunity_Triggered",
}


@dataclass(frozen=True)
class SpreadObservation:
    timestamp: str
    symbol: str
    gross_spread: float
    net_spread: float
    threshold: float
    threshold_reached: bool
    net_positive: bool
    opportunity_triggered: bool


@dataclass(frozen=True)
class SymbolSummary:
    symbol: str
    samples: int
    gross_mean: float
    gross_median: float
    gross_p90: float
    gross_p95: float
    gross_max: float
    threshold_hits: int
    positive_net: int
    opportunities: int
    first_timestamp: str
    last_timestamp: str


def _parse_bool(value, column, line_number):
    normalized = value.strip().lower()
    if normalized == "true":
        return True
    if normalized == "false":
        return False
    raise ValueError(f"line {line_number}: {column} must be true or false")


def _parse_finite_float(row, column, line_number):
    try:
        value = float(row[column])
    except (TypeError, ValueError) as error:
        raise ValueError(f"line {line_number}: {column} must be numeric") from error
    if not math.isfinite(value):
        raise ValueError(f"line {line_number}: {column} must be finite")
    return value


def read_observations(path):
    observations = []
    with Path(path).open("r", encoding="utf-8-sig", newline="") as source:
        reader = csv.DictReader(source)
        if reader.fieldnames is None:
            raise ValueError("CSV is empty or has no header")
        missing = REQUIRED_COLUMNS.difference(reader.fieldnames)
        if missing:
            raise ValueError(f"CSV is missing required columns: {', '.join(sorted(missing))}")

        for line_number, row in enumerate(reader, start=2):
            symbol = (row.get("Symbol") or "").strip()
            timestamp = (row.get("Timestamp_UTC") or "").strip()
            if not symbol or not timestamp:
                raise ValueError(f"line {line_number}: Symbol and Timestamp_UTC are required")
            threshold = _parse_finite_float(
                row, "Minimum_Trigger_Threshold_Percent", line_number
            )
            gross = _parse_finite_float(row, "Gross_Spread_Percent", line_number)
            net = _parse_finite_float(row, "Estimated_Net_Spread_Percent", line_number)
            if gross < 0 or threshold < 0:
                raise ValueError(f"line {line_number}: spreads and thresholds must be non-negative")
            observations.append(
                SpreadObservation(
                    timestamp=timestamp,
                    symbol=symbol,
                    gross_spread=gross,
                    net_spread=net,
                    threshold=threshold,
                    threshold_reached=_parse_bool(
                        row["Threshold_Reached"], "Threshold_Reached", line_number
                    ),
                    net_positive=_parse_bool(
                        row["Net_Spread_Positive"], "Net_Spread_Positive", line_number
                    ),
                    opportunity_triggered=_parse_bool(
                        row["Opportunity_Triggered"], "Opportunity_Triggered", line_number
                    ),
                )
            )
    if not observations:
        raise ValueError("CSV contains no spread observations")
    return observations


def _percentile(values, percentile):
    ordered = sorted(values)
    position = (len(ordered) - 1) * percentile
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    fraction = position - lower
    return ordered[lower] + (ordered[upper] - ordered[lower]) * fraction


def summarize_observations(observations):
    grouped = defaultdict(list)
    for observation in observations:
        grouped[observation.symbol].append(observation)

    summaries = []
    for symbol, rows in sorted(grouped.items()):
        spreads = [row.gross_spread for row in rows]
        summaries.append(
            SymbolSummary(
                symbol=symbol,
                samples=len(rows),
                gross_mean=statistics.fmean(spreads),
                gross_median=statistics.median(spreads),
                gross_p90=_percentile(spreads, 0.90),
                gross_p95=_percentile(spreads, 0.95),
                gross_max=max(spreads),
                threshold_hits=sum(row.threshold_reached for row in rows),
                positive_net=sum(row.net_positive for row in rows),
                opportunities=sum(row.opportunity_triggered for row in rows),
                first_timestamp=min(row.timestamp for row in rows),
                last_timestamp=max(row.timestamp for row in rows),
            )
        )
    return summaries


def write_summary(path, summaries):
    fieldnames = list(SymbolSummary.__dataclass_fields__)
    with Path(path).open("w", encoding="utf-8", newline="") as target:
        writer = csv.DictWriter(target, fieldnames=fieldnames)
        writer.writeheader()
        for summary in summaries:
            writer.writerow(summary.__dict__)


def _build_parser():
    parser = argparse.ArgumentParser(
        description="Summarize historical spread observations without connecting to exchanges."
    )
    parser.add_argument(
        "csv_file",
        nargs="?",
        default="spread_observations.csv",
        help="input observations CSV (default: spread_observations.csv)",
    )
    parser.add_argument(
        "--output",
        help="optional path for a per-symbol summary CSV",
    )
    return parser


def main(argv=None):
    args = _build_parser().parse_args(argv)
    console = Console()
    try:
        input_path = Path(args.csv_file)
        observations = read_observations(input_path)
        summaries = summarize_observations(observations)
        if args.output:
            output_path = Path(args.output)
            if input_path.resolve() == output_path.resolve():
                raise ValueError("summary output must not overwrite the observations CSV")
            write_summary(output_path, summaries)

        table = Table(title=f"Analyse des spreads · {input_path}")
        table.add_column("Paire", style="yellow")
        table.add_column("N", justify="right")
        table.add_column("Moyenne", justify="right")
        table.add_column("Médiane", justify="right")
        table.add_column("P90", justify="right")
        table.add_column("P95", justify="right")
        table.add_column("Maximum", justify="right")
        table.add_column("Seuil atteint", justify="right")
        table.add_column("Net positif", justify="right")
        table.add_column("Déclenché", justify="right")
        for summary in summaries:
            table.add_row(
                summary.symbol,
                str(summary.samples),
                f"{summary.gross_mean:.4f}%",
                f"{summary.gross_median:.4f}%",
                f"{summary.gross_p90:.4f}%",
                f"{summary.gross_p95:.4f}%",
                f"{summary.gross_max:.4f}%",
                f"{summary.threshold_hits}/{summary.samples}",
                f"{summary.positive_net}/{summary.samples}",
                str(summary.opportunities),
            )
        console.print(table)
        console.print(
            "Période observée : "
            f"{min(row.timestamp for row in observations)} à "
            f"{max(row.timestamp for row in observations)}"
        )
        console.print(
            "[dim]Les statistiques décrivent des observations historiques, "
            "pas une rentabilité exécutable.[/dim]"
        )
        if args.output:
            console.print(f"Synthèse CSV écrite dans : {args.output}")
        return 0
    except (OSError, ValueError) as error:
        Console(stderr=True).print(f"[bold red]Analyse impossible : {error}[/bold red]")
        return 2


if __name__ == "__main__":
    sys.exit(main())
