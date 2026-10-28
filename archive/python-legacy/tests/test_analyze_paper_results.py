import csv
import tempfile
import unittest
from pathlib import Path

from analyze_paper_results import summarize_paper_results


class AnalyzePaperResultsTests(unittest.TestCase):
    def test_summarizes_real_observation_status_and_opportunities(self):
        rows = [
            {"Market_Data_Status": "OK", "Opportunity_Triggered": "false"},
            {"Market_Data_Status": "OK", "Opportunity_Triggered": "true"},
            {"Market_Data_Status": "ANOMALY", "Opportunity_Triggered": "false"},
            {"Market_Data_Status": "STALE", "Opportunity_Triggered": "false"},
            {"Market_Data_Status": "LEGACY", "Opportunity_Triggered": ""},
        ]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "observations.csv"
            with path.open("w", encoding="utf-8", newline="") as target:
                writer = csv.DictWriter(
                    target,
                    fieldnames=["Market_Data_Status", "Opportunity_Triggered"],
                )
                writer.writeheader()
                writer.writerows(rows)

            summary = summarize_paper_results(path)

        self.assertEqual(summary.total_rows, 5)
        self.assertEqual(summary.valid_observations, 2)
        self.assertEqual(summary.triggered_opportunities, 1)
        self.assertEqual(summary.anomalies, 1)
        self.assertEqual(summary.other_statuses, 2)

    def test_requires_expected_status_column(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "observations.csv"
            path.write_text("status\nOK\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "Market_Data_Status"):
                summarize_paper_results(path)

    def test_rejects_invalid_opportunity_boolean_for_valid_rows(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "observations.csv"
            path.write_text(
                "Market_Data_Status,Opportunity_Triggered\nOK,yes\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(ValueError, "must be true or false"):
                summarize_paper_results(path)


if __name__ == "__main__":
    unittest.main()
