import argparse
import csv
import math
from datetime import datetime, timezone
from pathlib import Path
from typing import TypedDict


DEFAULT_LNG_CSV_PATH = Path("lng_observations.csv")
LNG_CSV_HEADERS = ["Timestamp_UTC", "Benchmark", "Price_USD", "Status", "Note"]


class LNGObservation(TypedDict):
    benchmark: str
    price: float
    status: str
    note: str


def fetch_lng_market_data() -> list[LNGObservation]:
    """Return clearly labelled sample values; no live market data is fetched."""
    return [
        {
            "benchmark": "TTF_Europe",
            "price": 34.50,
            "status": "SIMULATED",
            "note": "Exemple simulé uniquement; aucune cotation réelle ni donnée de transport.",
        },
        {
            "benchmark": "JKM_Asia",
            "price": 12.80,
            "status": "SIMULATED",
            "note": "Exemple simulé uniquement; aucune cotation réelle ni donnée de transport.",
        },
        {
            "benchmark": "HenryHub_US",
            "price": 2.25,
            "status": "SIMULATED",
            "note": "Exemple simulé uniquement; aucune cotation réelle ni donnée de transport.",
        },
    ]


def log_lng_observations(
    csv_path: str | Path = DEFAULT_LNG_CSV_PATH,
    *,
    observations: list[LNGObservation] | None = None,
    now: datetime | None = None,
) -> int:
    path = Path(csv_path)
    data = fetch_lng_market_data() if observations is None else observations
    timestamp = now or datetime.now(timezone.utc)
    if timestamp.tzinfo is None or timestamp.utcoffset() is None:
        raise ValueError("now must be a timezone-aware datetime")
    timestamp_text = timestamp.astimezone(timezone.utc).isoformat(
        timespec="seconds"
    ).replace("+00:00", "Z")

    rows = []
    for index, item in enumerate(data, start=1):
        benchmark = item["benchmark"]
        status = item["status"]
        note = item["note"]
        try:
            price = float(item["price"])
        except (TypeError, ValueError, OverflowError) as error:
            raise ValueError(f"observation {index}: price must be finite and positive") from error
        if not math.isfinite(price) or price <= 0:
            raise ValueError(f"observation {index}: price must be finite and positive")
        if not benchmark.strip() or not status.strip():
            raise ValueError(f"observation {index}: benchmark and status must not be empty")
        rows.append(
            {
                "Timestamp_UTC": timestamp_text,
                "Benchmark": benchmark,
                "Price_USD": price,
                "Status": status,
                "Note": note,
            }
        )

    path.parent.mkdir(parents=True, exist_ok=True)
    write_header = not path.exists() or path.stat().st_size == 0
    with path.open("a", encoding="utf-8", newline="") as target:
        writer = csv.DictWriter(target, fieldnames=LNG_CSV_HEADERS)
        if write_header:
            writer.writeheader()
        writer.writerows(rows)
    return len(rows)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(
        description="Write explicitly simulated LNG sample data to a CSV file."
    )
    parser.add_argument(
        "--csv",
        type=Path,
        default=DEFAULT_LNG_CSV_PATH,
        help="output CSV path (default: lng_observations.csv)",
    )
    args = parser.parse_args(argv)
    written = log_lng_observations(args.csv)
    print(
        f"{written} lignes d'exemple SIMULATED consignées dans {args.csv}; "
        "ce ne sont pas des données de marché réelles."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
