import argparse
import csv
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class PaperResultsSummary:
    total_rows: int
    valid_observations: int
    triggered_opportunities: int
    anomalies: int
    other_statuses: int


def summarize_paper_results(csv_path: str | Path) -> PaperResultsSummary:
    with Path(csv_path).open("r", encoding="utf-8-sig", newline="") as source:
        reader = csv.DictReader(source)
        if reader.fieldnames is None:
            raise ValueError("observation CSV is empty or has no header")
        if "Market_Data_Status" not in reader.fieldnames:
            raise ValueError("observation CSV is missing Market_Data_Status")

        total_rows = 0
        valid_observations = 0
        triggered_opportunities = 0
        anomalies = 0
        other_statuses = 0
        for line_number, row in enumerate(reader, start=2):
            total_rows += 1
            status = (row.get("Market_Data_Status") or "").strip().upper()
            if status == "ANOMALY":
                anomalies += 1
            elif status == "OK":
                valid_observations += 1
                triggered = (row.get("Opportunity_Triggered") or "").strip().lower()
                if triggered not in {"true", "false"}:
                    raise ValueError(
                        f"line {line_number}: Opportunity_Triggered must be true or false"
                    )
                triggered_opportunities += triggered == "true"
            else:
                other_statuses += 1

    return PaperResultsSummary(
        total_rows=total_rows,
        valid_observations=valid_observations,
        triggered_opportunities=triggered_opportunities,
        anomalies=anomalies,
        other_statuses=other_statuses,
    )


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(
        description="Summarize valid and anomalous market observations."
    )
    parser.add_argument(
        "csv_file",
        nargs="?",
        default="spread_observations.csv",
        help="observations CSV (default: spread_observations.csv)",
    )
    args = parser.parse_args(argv)
    csv_path = Path(args.csv_file)
    if not csv_path.exists():
        print(f"Aucun fichier d'observation trouvé : {csv_path}")
        return 1

    try:
        summary = summarize_paper_results(csv_path)
    except (OSError, csv.Error, UnicodeDecodeError, ValueError) as error:
        parser.error(str(error))

    print("--- Bilan des observations du simulateur papier ---")
    print(f"Total lignes lues : {summary.total_rows}")
    print(f"Observations valides (OK) : {summary.valid_observations}")
    print(f"Opportunités déclenchées (OK) : {summary.triggered_opportunities}")
    print(f"Anomalies filtrées : {summary.anomalies}")
    print(f"Autres états (STALE, DESYNC, LEGACY, etc.) : {summary.other_statuses}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
