import asyncio
from concurrent.futures import ThreadPoolExecutor
import importlib.util
from functools import partial
import os
from pathlib import Path
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import Mock, patch

from alert_gate import AlertGate
from market_data_health import assess_market_data_snapshot
from orderbook_execution import ExecutionEstimate


REPOSITORY = Path(__file__).resolve().parents[1]
MAIN_PATH = REPOSITORY / "main.py"


def load_main_without_starting_bot():
    temporary_directory = tempfile.TemporaryDirectory()
    previous_directory = Path.cwd()
    sys.path.insert(0, str(REPOSITORY))
    try:
        os.chdir(temporary_directory.name)
        environment = {
            "ALERT_CONFIRMATION_SECONDS": "0",
            "MARKET_DATA_STALE_SECONDS": "4",
            "NTFY_TOPIC_URL": "",
        }
        with patch.dict(os.environ, environment), patch.object(
            asyncio, "run", side_effect=lambda coroutine: coroutine.close()
        ):
            spec = importlib.util.spec_from_file_location(
                "_pravda_main_market_data_test", MAIN_PATH
            )
            module = importlib.util.module_from_spec(spec)
            sys.modules[spec.name] = module
            spec.loader.exec_module(module)
    finally:
        os.chdir(previous_directory)
        sys.path.remove(str(REPOSITORY))
    return module, temporary_directory


class HandlePriceUpdateFreshnessTests(unittest.IsolatedAsyncioTestCase):
    @classmethod
    def setUpClass(cls):
        cls.bot, cls.temporary_directory = load_main_without_starting_bot()

    @classmethod
    def tearDownClass(cls):
        cls.temporary_directory.cleanup()
        sys.modules.pop("_pravda_main_market_data_test", None)

    def setUp(self):
        self.symbol = "BTC/USDT"
        now = time.monotonic()
        self.bot.latest_prices = {
            "Kraken": {self.symbol: 100.0},
            "Binance": {self.symbol: 101.0},
        }
        self.bot.last_price_update_at = {
            "Kraken": {self.symbol: now},
            "Binance": {self.symbol: now},
        }
        self.bot.price_update_generation = {
            "Kraken": {self.symbol: 1},
            "Binance": {self.symbol: 1},
        }
        self.bot.last_movement_notification_at = {self.symbol: now}
        self.bot.last_spread_log_at = {}
        self.bot.alert_gate = AlertGate(0)
        self.bot.spread_summaries = {self.symbol: self.bot.SpreadSummary()}
        self.bot.opportunities_count = 0
        self.bot.paper_simulator = Mock()
        self.bot.paper_simulator.update.return_value = None
        self.bot.paper_simulator.next_position_notional_usdt = 100
        self.bot.fetch_public_execution_estimate = Mock(
            return_value=ExecutionEstimate(
                status="estimate_only",
                reason="test estimate",
                inventory_verified=False,
            )
        )
        self.bot.notify_opportunity = Mock()
        self.bot.ask_llm_analysis = Mock(return_value="analysis")
        self.bot.log_opportunity = Mock()
        self.bot.console.print = Mock()

        self.log_started = asyncio.Event()
        self.release_log = asyncio.Event()
        self.log_calls = 0
        self.logged_spread_rows = []
        self.execution_order = []
        self.movement_action_calls = []
        self.notification_started = asyncio.Event()
        self.release_notification = asyncio.Event()
        self.delay_notification = False
        self.analysis_started = asyncio.Event()
        self.release_analysis = asyncio.Event()
        self.delay_analysis = False
        thread_patch = patch.object(
            self.bot.asyncio, "to_thread", new=self.run_thread_call
        )
        thread_patch.start()
        self.addCleanup(thread_patch.stop)

    async def run_thread_call(self, function, *args, **kwargs):
        if function is self.bot.fetch_public_execution_estimate:
            return function(*args, **kwargs)
        if function is self.bot.log_paper_execution_estimate:
            return None
        if function is self.bot.log_spread_observation:
            self.log_calls += 1
            self.logged_spread_rows.append(args)
            self.execution_order.append(("spread_log", args[6]))
            if self.log_calls == 1:
                self.log_started.set()
                await self.release_log.wait()
            return None
        if function is self.bot.run_market_data_action:
            action = args[2]
            if action is self.bot.notify_opportunity:
                self.notification_started.set()
                if self.delay_notification:
                    await self.release_notification.wait()
            elif action is self.bot.ask_llm_analysis:
                self.analysis_started.set()
                if self.delay_analysis:
                    await self.release_analysis.wait()
            return function(*args, **kwargs)
        if function is self.bot.log_opportunity:
            return function(*args, **kwargs)
        if function is self.bot.notify_price_movement:
            self.movement_action_calls.append(args)
            return None
        raise AssertionError(f"Unexpected thread callback: {function!r}")

    async def start_triggering_tick(self):
        return asyncio.create_task(
            self.bot.handle_price_update("Kraken", self.symbol, 100.0)
        )

    async def test_stale_trigger_is_not_counted_or_logged_as_opportunity(self):
        with patch.object(
            self.bot,
            "current_market_data_health",
            return_value=self.bot.MarketDataHealth("STALE", 20, 0, 0),
        ):
            task = await self.start_triggering_tick()
            await task

        self.bot.paper_simulator.update.assert_not_called()
        self.bot.notify_opportunity.assert_not_called()
        self.bot.ask_llm_analysis.assert_not_called()
        self.assertFalse(self.bot.alert_gate.is_active(self.symbol))
        self.assertEqual(self.bot.opportunities_count, 0)
        self.assertEqual(self.bot.spread_summaries[self.symbol].opportunities, 0)
        self.assertFalse(any(row[6] for row in self.logged_spread_rows))

    async def test_new_anomalous_price_during_postsimulation_log_skips_notification(self):
        task = await self.start_triggering_tick()
        await self.log_started.wait()
        self.bot.paper_simulator.update.assert_called()
        await self.bot.handle_price_update("Binance", self.symbol, 200.0)
        self.release_log.set()
        await task

        self.bot.notify_opportunity.assert_not_called()
        self.bot.ask_llm_analysis.assert_not_called()
        self.assertFalse(self.bot.alert_gate.is_active(self.symbol))

    async def test_invalid_tick_during_postsimulation_log_skips_notification(self):
        task = await self.start_triggering_tick()
        await self.log_started.wait()
        self.bot.paper_simulator.update.assert_called()
        await self.bot.handle_price_update("Binance", self.symbol, "NaN")
        self.release_log.set()
        await task

        self.bot.notify_opportunity.assert_not_called()
        self.bot.ask_llm_analysis.assert_not_called()
        self.assertFalse(self.bot.alert_gate.is_active(self.symbol))

    async def test_old_exchange_event_does_not_refresh_feed_or_send_movement_alert(self):
        self.release_log.set()

        await self.bot.handle_price_update(
            "Binance",
            self.symbol,
            102.0,
            time.time() - 20,
        )

        self.assertGreater(
            time.monotonic() - self.bot.last_price_update_at["Binance"][self.symbol],
            19,
        )
        self.assertEqual(self.movement_action_calls, [])
        self.assertFalse(any(row[6] for row in self.logged_spread_rows))

    async def test_non_finite_movement_percentage_is_not_notified(self):
        self.release_log.set()
        self.bot.latest_prices["Kraken"][self.symbol] = 1e-308
        self.bot.last_price_update_at["Kraken"][self.symbol] = time.monotonic()
        self.bot.last_movement_notification_at[self.symbol] = 0

        await self.bot.handle_price_update("Kraken", self.symbol, 1.7e308)

        self.assertEqual(self.movement_action_calls, [])
        self.assertFalse(self.notification_started.is_set())

    async def test_ntfy_rechecks_freshness_after_waiting_for_global_lock(self):
        entered_lock = threading.Event()
        release_lock = threading.Event()

        class DelayedLock:
            def __enter__(self):
                entered_lock.set()
                if not release_lock.wait(timeout=5):
                    raise TimeoutError("test lock release timed out")
                return self

            def __exit__(self, *_):
                return False

        original_lock = self.bot.ntfy_lock
        original_blocked_until = self.bot.ntfy_blocked_until
        original_topic_url = self.bot.NTFY_TOPIC_URL
        self.bot.ntfy_lock = DelayedLock()
        self.bot.ntfy_blocked_until = 0
        self.bot.NTFY_TOPIC_URL = "https://ntfy.sh/test-topic"
        self.addCleanup(setattr, self.bot, "ntfy_lock", original_lock)
        self.addCleanup(setattr, self.bot, "ntfy_blocked_until", original_blocked_until)
        self.addCleanup(setattr, self.bot, "NTFY_TOPIC_URL", original_topic_url)

        loop = asyncio.get_running_loop()
        with patch.object(self.bot.requests, "post") as post:
            with ThreadPoolExecutor(max_workers=1) as executor:
                task = loop.run_in_executor(
                    executor,
                    partial(
                        self.bot.send_ntfy_notification,
                        "test",
                        "message",
                        pre_send_check=lambda: self.bot.is_market_data_snapshot_fresh(
                            (1, 1), self.symbol
                        ),
                    ),
                )
                while not entered_lock.is_set():
                    await asyncio.sleep(0.001)
                self.bot.last_price_update_at["Kraken"][self.symbol] = (
                    time.monotonic() - 20
                )
                release_lock.set()
                result = await task

            self.assertFalse(result)
            post.assert_not_called()

    async def test_stale_prices_after_ntfy_skip_llm(self):
        self.release_log.set()
        self.bot.notify_opportunity.side_effect = lambda *_: self.bot.last_price_update_at[
            "Kraken"
        ].__setitem__(self.symbol, time.monotonic() - 20)
        task = await self.start_triggering_tick()
        await task

        self.bot.paper_simulator.update.assert_called()
        self.bot.notify_opportunity.assert_called_once()
        self.bot.ask_llm_analysis.assert_not_called()
        self.assertTrue(self.bot.alert_gate.is_active(self.symbol))

    async def test_csv_trigger_row_is_written_after_paper_simulation(self):
        self.release_log.set()
        self.bot.paper_simulator.update.side_effect = lambda *args, **kwargs: (
            self.execution_order.append(("paper", kwargs.get("opportunity_confirmed", False)))
            or None
        )
        task = await self.start_triggering_tick()
        await task

        self.assertLess(
            self.execution_order.index(("paper", True)),
            self.execution_order.index(("spread_log", True)),
        )
        self.assertEqual(self.bot.opportunities_count, 1)
        self.assertEqual(self.bot.spread_summaries[self.symbol].opportunities, 1)

    async def test_queued_notification_skips_if_prices_expire_before_worker_starts(self):
        self.release_log.set()
        self.delay_notification = True
        task = await self.start_triggering_tick()
        await self.notification_started.wait()
        self.bot.last_price_update_at["Kraken"][self.symbol] = time.monotonic() - 20
        self.release_notification.set()
        await task

        self.bot.notify_opportunity.assert_not_called()
        self.bot.ask_llm_analysis.assert_not_called()
        self.assertFalse(self.bot.alert_gate.is_active(self.symbol))

    async def test_real_executor_rechecks_snapshot_when_queued_action_starts(self):
        loop = asyncio.get_running_loop()
        worker_started = threading.Event()
        release_worker = threading.Event()

        def occupy_executor():
            worker_started.set()
            release_worker.wait()

        with ThreadPoolExecutor(max_workers=1) as executor:
            blocker = loop.run_in_executor(executor, occupy_executor)
            while not worker_started.is_set():
                await asyncio.sleep(0.001)

            queued_action = loop.run_in_executor(
                executor,
                self.bot.run_market_data_action,
                (1, 1),
                self.symbol,
                self.bot.notify_opportunity,
                self.symbol,
                0.5,
            )
            self.bot.last_price_update_at["Kraken"][self.symbol] = (
                time.monotonic() - 20
            )
            release_worker.set()
            await blocker
            executed, _ = await queued_action

        self.assertFalse(executed)
        self.bot.notify_opportunity.assert_not_called()

    async def test_queued_llm_skips_if_prices_expire_before_worker_starts(self):
        self.release_log.set()
        self.delay_analysis = True
        task = await self.start_triggering_tick()
        await self.analysis_started.wait()
        self.bot.last_price_update_at["Kraken"][self.symbol] = time.monotonic() - 20
        self.release_analysis.set()
        await task

        self.bot.notify_opportunity.assert_called_once()
        self.bot.ask_llm_analysis.assert_not_called()
        self.assertTrue(self.bot.alert_gate.is_active(self.symbol))

    async def test_old_rejected_snapshot_does_not_cancel_newer_activation(self):
        task = await self.start_triggering_tick()
        await self.log_started.wait()
        await self.bot.handle_price_update("Binance", self.symbol, 100.1)
        await self.bot.handle_price_update("Binance", self.symbol, 101.0)
        newer_activation_id = self.bot.alert_gate.activation_id(self.symbol)
        self.assertIsNotNone(newer_activation_id)

        self.release_log.set()
        await task

        self.assertTrue(self.bot.alert_gate.is_active(self.symbol))
        self.assertEqual(
            self.bot.alert_gate.activation_id(self.symbol),
            newer_activation_id,
        )


if __name__ == "__main__":
    unittest.main()
