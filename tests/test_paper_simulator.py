import csv
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path

from paper_simulator import PaperSimulator


class PaperSimulatorTests(unittest.TestCase):
    def make_simulator(self, directory, mode="portfolio"):
        return PaperSimulator(
            mode=mode,
            starting_balance_usdt=1000,
            position_percent=10,
            fixed_position_usdt=100,
            exit_spread_percent=0.05,
            max_hold_seconds=3600,
            estimated_fees_percent=0.36,
            log_path=Path(directory) / "paper_trades.csv",
        )

    def test_portfolio_position_opens_closes_and_logs_estimated_pnl(self):
        with tempfile.TemporaryDirectory() as directory:
            simulator = self.make_simulator(directory)
            opened_at = datetime(2026, 1, 1, tzinfo=timezone.utc)
            opened = simulator.update(
                "BTC/USDT", 100.5, 100, opportunity_confirmed=True, now=opened_at
            )
            self.assertEqual(opened["action"], "opened")
            self.assertAlmostEqual(opened["notional_usdt"], 100)
            self.assertAlmostEqual(simulator.cash_usdt, 900)

            closed = simulator.update(
                "BTC/USDT",
                100.05,
                100,
                now=opened_at + timedelta(seconds=5),
            )
            self.assertEqual(closed["action"], "closed")
            self.assertAlmostEqual(closed["net_pnl_usdt"], 0.09, places=6)
            self.assertAlmostEqual(simulator.cash_usdt, 1000.09, places=6)
            with (Path(directory) / "paper_trades.csv").open(
                encoding="utf-8", newline=""
            ) as source:
                rows = list(csv.DictReader(source))
            self.assertEqual(rows[0]["Close_Reason"], "spread_target")

    def test_fixed_mode_caps_position_to_available_cash(self):
        with tempfile.TemporaryDirectory() as directory:
            simulator = self.make_simulator(directory, mode="fixed")
            simulator.cash_usdt = 40
            result = simulator.update(
                "ETH/USDT",
                100.5,
                100,
                opportunity_confirmed=True,
                now=datetime(2026, 1, 1, tzinfo=timezone.utc),
            )
            self.assertEqual(result["notional_usdt"], 40)
            self.assertEqual(simulator.cash_usdt, 0)

    def test_short_spread_direction_and_maximum_holding_exit(self):
        with tempfile.TemporaryDirectory() as directory:
            simulator = self.make_simulator(directory)
            opened_at = datetime(2026, 1, 1, tzinfo=timezone.utc)
            simulator.update(
                "ETH/USDT",
                99,
                100,
                opportunity_confirmed=True,
                now=opened_at,
            )
            closed = simulator.update(
                "ETH/USDT",
                99.95,
                100,
                now=opened_at + timedelta(seconds=1),
            )
            self.assertEqual(closed["reason"], "spread_target")
            self.assertAlmostEqual(closed["net_pnl_usdt"], 0.59, places=6)

            simulator.update(
                "SOL/USDT",
                100.5,
                100,
                opportunity_confirmed=True,
                now=opened_at + timedelta(seconds=2),
            )
            timed_out = simulator.update(
                "SOL/USDT",
                100.6,
                100,
                now=opened_at + timedelta(seconds=3602),
            )
            self.assertEqual(timed_out["reason"], "max_hold")

    def test_rejects_invalid_prices_and_naive_timestamps(self):
        with tempfile.TemporaryDirectory() as directory:
            simulator = self.make_simulator(directory)
            with self.assertRaisesRegex(ValueError, "Binance price"):
                simulator.update("BTC/USDT", 100, 0)
            with self.assertRaisesRegex(ValueError, "timezone"):
                simulator.update(
                    "BTC/USDT",
                    100,
                    99,
                    opportunity_confirmed=True,
                    now=datetime(2026, 1, 1),
                )


if __name__ == "__main__":
    unittest.main()
