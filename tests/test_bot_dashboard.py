import unittest

from bot_dashboard import SpreadSummary, build_dashboard


class BotDashboardTests(unittest.TestCase):
    def test_summary_tracks_spread_statistics(self):
        summary = SpreadSummary()
        summary.record(0.1, -0.2, True, False)
        summary.record(0.5, 0.1, True, True)
        self.assertEqual(summary.samples, 2)
        self.assertAlmostEqual(summary.gross_average, 0.3)
        self.assertEqual(summary.gross_max, 0.5)
        self.assertEqual(summary.threshold_hits, 2)
        self.assertEqual(summary.positive_net, 1)
        self.assertEqual(summary.opportunities, 1)

    def test_dashboard_handles_missing_market_data(self):
        table = build_dashboard(
            ["BTC/USDT"],
            {"Kraken": {}, "Binance": {}},
            {"BTC/USDT": SpreadSummary()},
            threshold=0.05,
            updates=0,
            opportunities=0,
        )
        self.assertEqual(table.row_count, 1)
        self.assertEqual(table.columns[0].header, "Paire")


if __name__ == "__main__":
    unittest.main()
