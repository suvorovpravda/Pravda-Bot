import csv
import tempfile
import unittest
from pathlib import Path

from csv_schema import ensure_csv_schema


class CsvSchemaTests(unittest.TestCase):
    def test_migrates_old_header_and_preserves_existing_rows(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "observations.csv"
            old_headers = ["Timestamp_UTC", "Symbol", "Gross_Spread_Percent"]
            with path.open("w", encoding="utf-8", newline="") as target:
                writer = csv.writer(target)
                writer.writerow(old_headers)
                writer.writerow(["2026-01-01T00:00:00Z", "BTC/USDT", "0.5"])

            ensure_csv_schema(
                path,
                [*old_headers, "Market_Data_Status"],
                added_column_defaults={"Market_Data_Status": "LEGACY"},
            )
            with path.open(encoding="utf-8", newline="") as source:
                reader = csv.DictReader(source)
                rows = list(reader)
                self.assertEqual(reader.fieldnames, [*old_headers, "Market_Data_Status"])
            self.assertEqual(rows[0]["Gross_Spread_Percent"], "0.5")
            self.assertEqual(rows[0]["Market_Data_Status"], "LEGACY")

    def test_rejects_incompatible_schema_without_changing_original(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "observations.csv"
            original = "Timestamp,Unknown\nold,data\n"
            path.write_text(original, encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "incompatible"):
                ensure_csv_schema(
                    path,
                    ["Timestamp", "New"],
                    added_column_defaults={"New": ""},
                )
            self.assertEqual(path.read_text(encoding="utf-8"), original)


if __name__ == "__main__":
    unittest.main()
