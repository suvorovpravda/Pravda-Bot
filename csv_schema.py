import csv
import os
import tempfile
from pathlib import Path
from collections.abc import Iterable, Mapping


def ensure_csv_schema(
    path: str | os.PathLike[str],
    fieldnames: Iterable[str],
    *,
    added_column_defaults: Mapping[str, str] | None = None,
) -> None:
    fieldnames = list(fieldnames)
    csv_path = Path(path)
    added_column_defaults = added_column_defaults or {}
    if not csv_path.exists() or csv_path.stat().st_size == 0:
        with csv_path.open("w", encoding="utf-8", newline="") as target:
            csv.writer(target).writerow(fieldnames)
        return

    temporary_path = None
    try:
        with csv_path.open("r", encoding="utf-8-sig", newline="") as source:
            reader = csv.DictReader(source)
            existing_fields = reader.fieldnames
            if existing_fields == list(fieldnames):
                return
            if existing_fields is None:
                raise ValueError(f"{csv_path} has no readable CSV header")

            new_fields = set(fieldnames).difference(existing_fields)
            removed_fields = set(existing_fields).difference(fieldnames)
            if removed_fields or not new_fields.issubset(added_column_defaults):
                raise ValueError(
                    f"{csv_path} has an incompatible CSV schema; preserve the file and migrate it manually"
                )

            with tempfile.NamedTemporaryFile(
                "w",
                encoding="utf-8",
                newline="",
                dir=str(csv_path.parent),
                prefix=f".{csv_path.name}.",
                suffix=".tmp",
                delete=False,
            ) as target:
                temporary_path = target.name
                writer = csv.DictWriter(target, fieldnames=fieldnames)
                writer.writeheader()
                for row in reader:
                    migrated = {
                        column: row.get(column, added_column_defaults.get(column, ""))
                        for column in fieldnames
                    }
                    writer.writerow(migrated)
        if temporary_path is None:
            raise RuntimeError("CSV schema migration did not create a temporary file")
        os.replace(temporary_path, csv_path)
    finally:
        if temporary_path and os.path.exists(temporary_path):
            os.remove(temporary_path)
