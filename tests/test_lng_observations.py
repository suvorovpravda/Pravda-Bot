import csv
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path

from lng_observations import (
    LNG_CSV_HEADERS,
    fetch_lng_market_data,
    log_lng_observations,
)


class LNGObservationsTests(unittest.TestCase):
    def test_sample_values_are_marked_simulated_and_logged_as_utc(self):
        fixed_time = datetime(2026, 1, 1, 1, 0, tzinfo=timezone.utc)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "lng.csv"
            count = log_lng_observations(path, now=fixed_time)

            with path.open(encoding="utf-8", newline="") as source:
                reader = csv.DictReader(source)
                rows = list(reader)

        self.assertEqual(count, 3)
        self.assertEqual(reader.fieldnames, LNG_CSV_HEADERS)
        self.assertEqual({row["Status"] for row in rows}, {"SIMULATED"})
        self.assertEqual(
            {row["Timestamp_UTC"] for row in rows},
            {"2026-01-01T01:00:00Z"},
        )
        self.assertTrue(all("simulé" in row["Note"] for row in rows))

    def test_appends_without_repeating_the_header(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "lng.csv"
            log_lng_observations(path)
            log_lng_observations(path)
            with path.open(encoding="utf-8", newline="") as source:
                lines = source.readlines()

        self.assertEqual(sum(line.startswith("Timestamp_UTC,") for line in lines), 1)
        self.assertEqual(len(lines), 7)

    def test_rejects_invalid_prices_before_writing_any_rows(self):
        invalid_observations = [
            {
                "benchmark": "TEST",
                "price": value,
                "status": "SIMULATED",
                "note": "test",
            }
            for value in (0, -1, float("nan"), float("inf"), "invalid")
        ]
        for observation in invalid_observations:
            with self.subTest(price=observation["price"]):
                with tempfile.TemporaryDirectory() as directory:
                    path = Path(directory) / "lng.csv"
                    with self.assertRaisesRegex(ValueError, "finite and positive"):
                        log_lng_observations(
                            path,
                            observations=[observation],
                        )
                    self.assertFalse(path.exists())

    def test_rejects_naive_timestamp(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "lng.csv"
            with self.assertRaisesRegex(ValueError, "timezone-aware"):
                log_lng_observations(path, now=datetime(2026, 1, 1))
            self.assertFalse(path.exists())

    def test_fixture_contains_only_the_expected_benchmarks(self):
        self.assertEqual(
            {row["benchmark"] for row in fetch_lng_market_data()},
            {"TTF_Europe", "JKM_Asia", "HenryHub_US"},
        )


if __name__ == "__main__":
    unittest.main()
