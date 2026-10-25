import csv
import tempfile
import unittest
from pathlib import Path

from analyze_spreads import (
    REQUIRED_COLUMNS,
    SpreadObservation,
    read_observations,
    summarize_observations,
    write_summary,
)


def observation(timestamp, symbol, gross, net, threshold, threshold_reached, net_positive, triggered):
    return {
        "Timestamp_UTC": timestamp,
        "Symbol": symbol,
        "Kraken_Price": "100",
        "Binance_Price": "101",
        "Gross_Spread_Percent": str(gross),
        "Estimated_Net_Spread_Percent": str(net),
        "Minimum_Trigger_Threshold_Percent": str(threshold),
        "Estimated_Fees_Percent": "0.36",
        "Threshold_Reached": str(threshold_reached).lower(),
        "Net_Spread_Positive": str(net_positive).lower(),
        "Opportunity_Triggered": str(triggered).lower(),
    }


class AnalyzeSpreadsTests(unittest.TestCase):
    def write_csv(self, path, rows, headers=None):
        headers = headers or sorted(REQUIRED_COLUMNS | {
            "Kraken_Price",
            "Binance_Price",
            "Estimated_Fees_Percent",
        })
        with path.open("w", encoding="utf-8", newline="") as target:
            writer = csv.DictWriter(target, fieldnames=headers)
            writer.writeheader()
            writer.writerows(rows)

    def test_reads_and_summarizes_percentiles_per_symbol(self):
        rows = [
            observation("2026-01-01T00:00:00Z", "BTC/USDT", 1, 0.5, 0.05, True, True, False),
            observation("2026-01-01T00:00:10Z", "BTC/USDT", 2, -0.1, 0.05, True, False, True),
            observation("2026-01-01T00:00:20Z", "BTC/USDT", 3, 0.1, 0.05, True, True, False),
            observation("2026-01-01T00:00:00Z", "ETH/USDT", 4, -1, 0.05, True, False, False),
        ]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "spreads.csv"
            self.write_csv(path, rows)
            summaries = summarize_observations(read_observations(path))
            btc = summaries[0]
            self.assertEqual(btc.symbol, "BTC/USDT")
            self.assertEqual(btc.samples, 3)
            self.assertEqual(btc.gross_mean, 2)
            self.assertEqual(btc.gross_median, 2)
            self.assertAlmostEqual(btc.gross_p90, 2.8)
            self.assertEqual(btc.threshold_hits, 3)
            self.assertEqual(btc.positive_net, 2)
            self.assertEqual(btc.opportunities, 1)

    def test_excludes_stale_market_data_from_spread_statistics(self):
            rows = [
                observation("2026-01-01T00:00:00Z", "BTC/USDT", 0.1, -0.2, 0.05, True, False, False),
                observation("2026-01-01T00:00:10Z", "BTC/USDT", 0.8, 0.4, 0.05, True, True, True),
            ]
            rows[0]["Market_Data_Status"] = "OK"
            rows[1]["Market_Data_Status"] = "STALE"
            headers = sorted(REQUIRED_COLUMNS | {
                "Kraken_Price",
                "Binance_Price",
                "Estimated_Fees_Percent",
                "Market_Data_Status",
            })
            with tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / "spreads.csv"
                self.write_csv(path, rows, headers)
                summary = summarize_observations(read_observations(path))[0]
                self.assertEqual(summary.samples, 1)
                self.assertEqual(summary.gross_max, 0.1)
                self.assertEqual(summary.excluded_market_data, 1)

    def test_rejects_missing_columns_invalid_boolean_and_nonfinite_values(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "spreads.csv"
            path.write_text("Symbol\nBTC/USDT\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "missing required columns"):
                read_observations(path)

            valid = observation("2026-01-01T00:00:00Z", "BTC/USDT", 1, 0, 0.05, True, False, False)
            valid["Threshold_Reached"] = "yes"
            self.write_csv(path, [valid])
            with self.assertRaisesRegex(ValueError, "must be true or false"):
                read_observations(path)

            valid["Threshold_Reached"] = "true"
            valid["Gross_Spread_Percent"] = "inf"
            self.write_csv(path, [valid])
            with self.assertRaisesRegex(ValueError, "must be finite"):
                read_observations(path)

    def test_writes_summary_csv(self):
        summaries = summarize_observations(
            [
                SpreadObservation(
                    timestamp="2026-01-01T00:00:00Z",
                    symbol="BTC/USDT",
                    gross_spread=0.1,
                    net_spread=-0.26,
                    threshold=0.05,
                    threshold_reached=True,
                    net_positive=False,
                    opportunity_triggered=False,
                    market_data_status="OK",
                )
            ]
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "summary.csv"
            write_summary(path, summaries)
            with path.open(encoding="utf-8", newline="") as source:
                rows = list(csv.DictReader(source))
            self.assertEqual(len(rows), 1)
            self.assertEqual(rows[0]["symbol"], "BTC/USDT")
            self.assertEqual(rows[0]["samples"], "1")


if __name__ == "__main__":
    unittest.main()
