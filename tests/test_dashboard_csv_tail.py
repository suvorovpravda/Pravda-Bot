import csv
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

import dashboard


class ObservationTailTests(unittest.TestCase):
    def setUp(self):
        self.folder = TemporaryDirectory()
        self.addCleanup(self.folder.cleanup)
        self.path = Path(self.folder.name) / "observations.csv"

    def rows(self, keep=20):
        return list(dashboard._read_observation_rows(
            self.path, keep=keep, required_columns={"Timestamp_UTC", "Symbol"}))

    def test_50001_rows_keep_recent_observations(self):
        with self.path.open("w", encoding="utf-8", newline="") as target:
            writer = csv.writer(target)
            writer.writerow(["Timestamp_UTC", "Symbol"])
            for index in range(50001):
                writer.writerow(["2026-10-27T00:00:00Z", str(index)])
        rows = self.rows()
        self.assertEqual([row["Symbol"] for row in rows], [str(i) for i in range(49981, 50001)])

    def test_log_over_eight_mib_retains_latest_rows_with_bounded_io(self):
        with self.path.open("wb") as target:
            target.write(b"Timestamp_UTC,Symbol,Note\n")
            target.write((b"2026-10-27T00:00:00Z,BTC/USDT," + b"x" * 120 + b"\n") * 65000)
            target.write(b"2026-10-27T01:00:00Z,ETH/USDT,newest\n")
        self.assertGreater(self.path.stat().st_size, 8 * 1024 * 1024)
        original_fdopen = dashboard.os.fdopen
        read_sizes = []

        class TrackedReader:
            def __init__(self, source):
                self.source = source

            def __enter__(self):
                self.source.__enter__()
                return self

            def __exit__(self, *args):
                return self.source.__exit__(*args)

            def __getattr__(self, name):
                return getattr(self.source, name)

            def read(self, size):
                self_check.assertGreater(size, 0)
                data = self.source.read(size)
                read_sizes.append(len(data))
                return data

            def readline(self, size):
                data = self.source.readline(size)
                read_sizes.append(len(data))
                return data

        self_check = self
        with patch("dashboard.os.fdopen", side_effect=lambda *a, **kw: TrackedReader(original_fdopen(*a, **kw))):
            rows = self.rows()
        self.assertEqual(len(rows), 20)
        self.assertEqual(rows[-1]["Symbol"], "ETH/USDT")
        self.assertEqual(rows[-1]["Note"], "newest")
        self.assertLessEqual(sum(read_sizes), dashboard.MAX_CSV_BYTES)
        self.assertLess(sum(read_sizes), 128 * 1024)

    def test_tail_preserves_quoted_newlines_escaped_quotes_crlf_and_utf8(self):
        expected = []
        with self.path.open("w", encoding="utf-8-sig", newline="") as target:
            writer = csv.writer(target)
            writer.writerow(["Timestamp_UTC", "Symbol", "Note"])
            for index in range(2000):
                note = f'Note {index}, avec "citation"\nété\r\nseconde ligne'
                writer.writerow(["2026-10-27T00:00:00Z", str(index), note])
                expected.append({"Timestamp_UTC": "2026-10-27T00:00:00Z", "Symbol": str(index), "Note": note})
        with patch("dashboard.MAX_CSV_BYTES", 4096), patch("dashboard.CSV_TAIL_CHUNK_BYTES", 17):
            self.assertEqual(self.rows(), expected[-20:])

    def test_lng_multiline_record_is_not_split_into_observations(self):
        with self.path.open("w", encoding="utf-8", newline="") as target:
            writer = csv.writer(target)
            writer.writerow(["Timestamp_UTC", "Benchmark", "Price_USD", "Status", "Note"])
            for index in range(1000):
                writer.writerow(["2026-10-27T00:00:00Z", str(index), 12, "SIMULATED", 'note\n"suivante"'])
        with patch("dashboard.MAX_CSV_BYTES", 2048):
            rows = dashboard.read_lng_dashboard_data(self.path)["observations"]
        self.assertEqual(len(rows), 10)
        self.assertEqual(rows[0]["Benchmark"], "999")
        self.assertEqual(rows[0]["Note"], 'note\n"suivante"')

    def test_single_oversized_record_and_invalid_schema_still_fail(self):
        self.path.write_text("Timestamp_UTC,Symbol\nnow," + "x" * 256 + "\n", encoding="utf-8")
        with patch("dashboard.MAX_CSV_BYTES", 128), self.assertRaisesRegex(ValueError, "byte limit"):
            self.rows()
        self.path.write_text("Timestamp,Status\nnow,OK\n", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "schema"):
            self.rows()


if __name__ == "__main__":
    unittest.main()
