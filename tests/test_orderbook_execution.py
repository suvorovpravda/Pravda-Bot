import unittest
from datetime import datetime, timedelta, timezone
from unittest.mock import patch

import ccxt
from orderbook_execution import (
    BookSnapshot,
    MarketRules,
    estimate_cross_exchange_execution,
    fetch_public_execution_estimate,
)


class OrderBookExecutionTests(unittest.TestCase):
    def setUp(self):
        self.now = datetime(2026, 1, 1, tzinfo=timezone.utc)
        self.books = {
            "Kraken": BookSnapshot(
                "Kraken",
                bids=[[101, 2], [100.9, 4]],
                asks=[[101.1, 2], [101.2, 4]],
                received_at=self.now,
            ),
            "Binance": BookSnapshot(
                "Binance",
                bids=[[100, 2], [99.9, 4]],
                asks=[[100.1, 2], [100.2, 4]],
                received_at=self.now,
            ),
        }
        self.rules = {
            name: MarketRules(min_amount=0.001, min_cost=5, amount_step=0.001)
            for name in self.books
        }

    def estimate(self, **overrides):
        arguments = {
            "target_notional": 100,
            "taker_fee_percent": 0.1,
            "max_slippage_percent": 0.5,
            "max_book_age_seconds": 2,
            "now": self.now,
        }
        arguments.update(overrides)
        return estimate_cross_exchange_execution(self.books, self.rules, **arguments)

    def test_sweeps_depth_and_calculates_two_leg_vwap_fees_and_pnl(self):
        result = self.estimate()
        self.assertEqual(result.status, "estimate_only")
        self.assertEqual(result.buy.exchange, "Binance")
        self.assertEqual(result.sell.exchange, "Kraken")
        self.assertAlmostEqual(result.buy.amount, 0.999)
        self.assertAlmostEqual(result.buy.average_price, 100.1)
        self.assertAlmostEqual(result.sell.average_price, 101)
        self.assertAlmostEqual(result.fees, 0.2008989)
        self.assertGreater(result.net_pnl, 0)
        self.assertFalse(result.inventory_verified)

    def test_partial_depth_is_marked_and_not_treated_as_full_execution(self):
        self.books["Kraken"] = BookSnapshot(
            "Kraken", bids=[[101, 0.2]], asks=[[101.1, 1]], received_at=self.now
        )
        result = self.estimate()
        self.assertEqual(result.status, "partial_estimate")
        self.assertIn("partial", result.reason)
        self.assertAlmostEqual(result.sell.amount, 0.2)

    def test_stale_books_and_minimum_order_rules_reject(self):
        self.books["Kraken"] = BookSnapshot(
            "Kraken",
            self.books["Kraken"].bids,
            self.books["Kraken"].asks,
            self.now - timedelta(seconds=3),
        )
        stale = self.estimate()
        self.assertEqual(stale.status, "rejected")
        self.assertIn("stale", stale.reason)

        self.books["Kraken"] = BookSnapshot(
            "Kraken",
            bids=[[101, 2]],
            asks=[[101.1, 2]],
            received_at=self.now,
        )
        self.rules["Binance"] = MarketRules(min_amount=2, min_cost=5, amount_step=0.001)
        too_small = self.estimate()
        self.assertEqual(too_small.status, "rejected")
        self.assertIn("minimum amount", too_small.reason)

    def test_slippage_limit_and_unprofitable_net_return_reject(self):
        self.books["Kraken"] = BookSnapshot(
            "Kraken", bids=[[101, 0.1], [100, 2]], asks=[[101.1, 2]], received_at=self.now
        )
        slipped = self.estimate(max_slippage_percent=0.01)
        self.assertEqual(slipped.status, "rejected")
        self.assertEqual(slipped.reason, "slippage limit exceeded")

        no_profit = self.estimate(taker_fee_percent=2, max_slippage_percent=10)
        self.assertEqual(no_profit.status, "rejected")
        self.assertEqual(no_profit.reason, "estimated net profit is not positive")

    def test_rejects_invalid_market_rules(self):
        self.rules["Kraken"] = MarketRules(amount_step=-1)
        with self.assertRaisesRegex(ValueError, "amount step"):
            self.estimate()

    def test_market_maximums_and_significant_digit_precision_are_enforced(self):
        self.rules["Kraken"] = MarketRules(
            min_amount=0.001,
            min_cost=5,
            max_amount=0.5,
            amount_significant_digits=3,
        )
        too_large = self.estimate()
        self.assertEqual(too_large.status, "rejected")
        self.assertIn("maximum amount", too_large.reason)

        self.rules["Kraken"] = MarketRules(
            min_amount=0.001,
            min_cost=5,
            amount_significant_digits=2,
        )
        rounded = self.estimate()
        self.assertEqual(rounded.status, "estimate_only")
        self.assertAlmostEqual(rounded.buy.amount, 0.99)

    def test_fetches_public_books_and_exchange_market_limits(self):
        class FakeExchange:
            precisionMode = ccxt.TICK_SIZE

            def load_markets(self):
                return {
                    "BTC/USDT": {
                        "limits": {
                            "amount": {"min": 0.001},
                            "cost": {"min": 5},
                        },
                        "precision": {"amount": 0.001},
                    }
                }

            def fetch_order_book(self, symbol, *, limit):
                self.asserted_symbol = symbol
                self.asserted_limit = limit
                if symbol == "BTC/USDT":
                    return {
                        "bids": [[101, 2]],
                        "asks": [[101.1, 2]],
                    }
                return {}

            def close(self):
                return None

        kraken = FakeExchange()
        binance = FakeExchange()
        binance.fetch_order_book = lambda symbol, *, limit: {
            "bids": [[100, 2]],
            "asks": [[100.1, 2]],
        }
        with patch.object(ccxt, "kraken", return_value=kraken), patch.object(
            ccxt, "binance", return_value=binance
        ):
            result = fetch_public_execution_estimate(
                "BTC/USDT",
                target_notional=100,
                taker_fee_percent=0.1,
                max_slippage_percent=0.5,
                max_book_age_seconds=2,
            )
        self.assertEqual(result.status, "estimate_only")
        self.assertEqual(result.buy.exchange, "Binance")
        self.assertFalse(result.inventory_verified)


if __name__ == "__main__":
    unittest.main()
