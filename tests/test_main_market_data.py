import asyncio
import importlib.util
import os
from pathlib import Path
import sys
import tempfile
import time
import unittest
from unittest.mock import Mock, patch

from alert_gate import AlertGate
from market_data_health import assess_market_data_snapshot


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
        self.bot.notify_opportunity = Mock()
        self.bot.ask_llm_analysis = Mock(return_value="analysis")
        self.bot.log_opportunity = Mock()
        self.bot.console.print = Mock()

        self.log_started = asyncio.Event()
        self.release_log = asyncio.Event()
        self.log_calls = 0
        self.notification_started = asyncio.Event()
        self.release_notification = asyncio.Event()
        self.delay_notification = False
        thread_patch = patch.object(
            self.bot.asyncio, "to_thread", new=self.run_thread_call
        )
        thread_patch.start()
        self.addCleanup(thread_patch.stop)

    async def run_thread_call(self, function, *args, **kwargs):
        if function is self.bot.log_spread_observation:
            self.log_calls += 1
            if self.log_calls == 1:
                self.log_started.set()
                await self.release_log.wait()
            return None
        if function is self.bot.notify_opportunity:
            self.notification_started.set()
            if self.delay_notification:
                await self.release_notification.wait()
            return None
        if function is self.bot.ask_llm_analysis:
            return function(*args, **kwargs)
        if function is self.bot.log_opportunity:
            return function(*args, **kwargs)
        if function is self.bot.notify_price_movement:
            return None
        raise AssertionError(f"Unexpected thread callback: {function!r}")

    async def start_triggering_tick(self):
        return asyncio.create_task(
            self.bot.handle_price_update("Kraken", self.symbol, 100.0)
        )

    async def test_twenty_second_stale_log_skips_paper_notification_and_llm(self):
        original_check = self.bot.assess_market_data_snapshot

        def stale_after_log(snapshot, current, kraken_updated_at, binance_updated_at, **kwargs):
            return original_check(
                snapshot,
                current,
                kraken_updated_at,
                binance_updated_at,
                now=time.monotonic() + 20,
                **kwargs,
            )

        with patch.object(
            self.bot, "assess_market_data_snapshot", side_effect=stale_after_log
        ):
            task = await self.start_triggering_tick()
            await self.log_started.wait()
            self.release_log.set()
            await task

        self.bot.paper_simulator.update.assert_not_called()
        self.assertFalse(self.notification_started.is_set())
        self.bot.ask_llm_analysis.assert_not_called()
        self.assertFalse(self.bot.alert_gate.is_active(self.symbol))

    async def test_new_anomalous_price_during_log_abandons_old_snapshot(self):
        old_binance_generation = self.bot.price_update_generation["Binance"][self.symbol]
        task = await self.start_triggering_tick()
        await self.log_started.wait()
        await self.bot.handle_price_update("Binance", self.symbol, 200.0)
        self.assertNotEqual(
            old_binance_generation,
            self.bot.price_update_generation["Binance"][self.symbol],
        )
        with patch.object(
            self.bot,
            "assess_market_data_snapshot",
            wraps=assess_market_data_snapshot,
        ) as snapshot_check:
            self.release_log.set()
            await task

        snapshot_check.assert_called_once()
        self.assertNotEqual(
            snapshot_check.call_args.args[0],
            snapshot_check.call_args.args[1],
        )
        self.bot.paper_simulator.update.assert_not_called()
        self.assertFalse(self.notification_started.is_set())
        self.bot.ask_llm_analysis.assert_not_called()
        self.assertFalse(self.bot.alert_gate.is_active(self.symbol))

    async def test_invalid_tick_during_log_abandons_old_snapshot(self):
        task = await self.start_triggering_tick()
        await self.log_started.wait()
        await self.bot.handle_price_update("Binance", self.symbol, "NaN")
        self.release_log.set()
        await task

        self.bot.paper_simulator.update.assert_not_called()
        self.assertFalse(self.notification_started.is_set())
        self.bot.ask_llm_analysis.assert_not_called()
        self.assertFalse(self.bot.alert_gate.is_active(self.symbol))

    async def test_stale_prices_after_ntfy_skip_llm(self):
        original_check = self.bot.assess_market_data_snapshot
        check_count = 0

        def stale_after_notification(snapshot, current, kraken_updated_at, binance_updated_at, **kwargs):
            nonlocal check_count
            check_count += 1
            now = time.monotonic() + (20 if check_count > 1 else 0)
            return original_check(
                snapshot,
                current,
                kraken_updated_at,
                binance_updated_at,
                now=now,
                **kwargs,
            )

        self.release_log.set()
        with patch.object(
            self.bot,
            "assess_market_data_snapshot",
            side_effect=stale_after_notification,
        ):
            self.delay_notification = True
            task = await self.start_triggering_tick()
            await self.notification_started.wait()
            self.release_notification.set()
            await task

        self.bot.paper_simulator.update.assert_called()
        self.assertTrue(self.notification_started.is_set())
        self.bot.ask_llm_analysis.assert_not_called()

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
