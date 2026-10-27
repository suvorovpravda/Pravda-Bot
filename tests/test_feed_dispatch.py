import asyncio
from concurrent.futures import ThreadPoolExecutor
from contextlib import suppress
from dataclasses import FrozenInstanceError, replace
from datetime import datetime, timedelta, timezone
import json
from pathlib import Path
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import Mock, patch

from alert_gate import AlertGate
from orderbook_execution import ExecutionEstimate
from tests.test_main_market_data import load_main_without_starting_bot


REAL_TO_THREAD = asyncio.to_thread


class FakeWebSocket:
    def __init__(self):
        self.frames = asyncio.Queue()
        self.reads = asyncio.Queue()

    async def __aenter__(self):
        return self

    async def __aexit__(self, *_):
        return False

    async def send(self, _message):
        pass

    def __aiter__(self):
        return self

    async def __anext__(self):
        frame = await self.frames.get()
        if frame is None:
            raise StopAsyncIteration
        self.reads.put_nowait(frame)
        return frame

    def push(self, exchange, symbol, price, timestamp=None):
        epoch = time.time() if timestamp is None else timestamp
        if exchange == "Kraken":
            payload = {"channel": "ticker", "data": [{"symbol": symbol, "last": price,
                       "timestamp": datetime.fromtimestamp(epoch, timezone.utc).isoformat()}]}
        else:
            payload = {"data": {"s": symbol.replace("/", ""), "c": str(price), "E": int(epoch * 1000)}}
        self.frames.put_nowait(json.dumps(payload))


class FeedDispatchTests(unittest.IsolatedAsyncioTestCase):
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
        self.bot.latest_prices = {"Kraken": {self.symbol: 100.0}, "Binance": {self.symbol: 101.0}}
        self.bot.last_price_update_at = {name: {self.symbol: now} for name in ("Kraken", "Binance")}
        self.bot.price_update_generation = {name: {self.symbol: 1} for name in ("Kraken", "Binance")}
        self.bot.last_movement_notification_at = {self.symbol: now}
        self.bot.last_spread_log_at = {}
        self.bot.spread_summaries = {symbol: self.bot.SpreadSummary() for symbol in self.bot.SYMBOLS}
        self.bot.alert_gate = AlertGate(0)
        self.bot.opportunities_count = 0
        self.bot.price_updates_count = 0
        for name in ("market_store", "paper_simulator", "console", "log_spread_observation",
                     "log_paper_execution_estimate", "log_opportunity", "notify_opportunity",
                     "notify_price_movement", "write_market_snapshot"):
            patcher = patch.object(self.bot, name, Mock())
            patcher.start()
            self.addCleanup(patcher.stop)
        self.bot.paper_simulator.update.return_value = None
        self.bot.paper_simulator.next_position_notional_usdt = 100
        for name, replacement in (
            ("fetch_public_execution_estimate", Mock(return_value=ExecutionEstimate(
                status="estimate_only", reason="test estimate"))),
            ("ask_llm_analysis", Mock(return_value="analysis")),
        ):
            patcher = patch.object(self.bot, name, replacement)
            patcher.start()
            self.addCleanup(patcher.stop)

    async def wait_thread(self, event):
        self.assertTrue(await REAL_TO_THREAD(event.wait, 2), "worker never reached barrier")

    async def stop(self, task):
        task.cancel()
        with suppress(asyncio.CancelledError):
            await task

    async def check_reader(self, exchange, *, block_history=False):
        entered, release = threading.Event(), threading.Event()
        owner_thread = threading.get_ident()
        call_threads = []

        def block(*_, **__):
            call_threads.append(threading.get_ident())
            entered.set()
            release.wait(0.5 if threading.get_ident() == owner_thread else 3)
            return "analysis"

        target = (patch.object(self.bot.market_store, "record_tick", side_effect=block)
                  if block_history else patch.object(self.bot, "ask_llm_analysis", side_effect=block))
        socket = FakeWebSocket()
        stream = self.bot.stream_kraken_tickers if exchange == "Kraken" else self.bot.stream_binance_tickers
        first_price = 100 if exchange == "Kraken" else 101
        with patch.object(self.bot.websockets, "connect", return_value=socket), target:
            task = asyncio.create_task(stream())
            try:
                socket.push(exchange, self.symbol, first_price)
                await self.wait_thread(entered)
                await socket.reads.get()
                epoch = time.time() - 0.25
                socket.push(exchange, self.symbol, first_price + 0.1, epoch)
                socket.push(exchange, "ETH/USDT", 2000, epoch)
                await asyncio.wait_for(socket.reads.get(), 0.4)
                await asyncio.wait_for(socket.reads.get(), 0.4)
                self.assertEqual(self.bot.latest_prices[exchange][self.symbol], first_price + 0.1)
                self.assertEqual(self.bot.latest_prices[exchange]["ETH/USDT"], 2000)
                self.assertEqual(self.bot.price_update_generation[exchange][self.symbol], 3)
                self.assertLess(abs(time.monotonic() - self.bot.last_price_update_at[exchange][self.symbol] - 0.25), 0.2)
                self.assertNotIn(owner_thread, call_threads)
                self.assertFalse(release.is_set())
            finally:
                release.set()
                await self.stop(task)

    async def test_kraken_reader_continues_during_llm(self):
        await self.check_reader("Kraken")

    async def test_binance_reader_continues_during_llm(self):
        await self.check_reader("Binance")

    async def test_sqlite_writer_does_not_block_reader(self):
        await self.check_reader("Kraken", block_history=True)

    async def test_prices_expired_during_orderbooks_never_reach_paper_again(self):
        entered, release = threading.Event(), threading.Event()
        estimate = self.bot.fetch_public_execution_estimate.return_value

        def books(*_, **__):
            entered.set()
            release.wait(3)
            return estimate

        with patch.object(self.bot, "fetch_public_execution_estimate", side_effect=books):
            task = asyncio.create_task(self.bot.handle_price_update("Kraken", self.symbol, 100))
            try:
                await self.wait_thread(entered)
                self.bot.last_price_update_at["Kraken"][self.symbol] = time.monotonic() - 20
                release.set()
                await task
            finally:
                release.set()
                await self.stop(task)
        self.assertEqual([call for call in self.bot.paper_simulator.update.call_args_list
                          if call.kwargs.get("opportunity_confirmed")], [])
        self.assertEqual(self.bot.opportunities_count, 0)
        self.assertFalse(any(call.args[6] for call in self.bot.log_spread_observation.call_args_list))

    async def test_opportunity_log_rechecks_generation_at_executor_start(self):
        loop = asyncio.get_running_loop()
        log_queued, executor_blocked = asyncio.Event(), asyncio.Event()
        release = threading.Event()
        blockers = []

        def block():
            loop.call_soon_threadsafe(executor_blocked.set)
            release.wait(3)

        with ThreadPoolExecutor(max_workers=1) as executor:
            loop.set_default_executor(executor)

            def enqueue_blocker():
                blockers.append(loop.run_in_executor(executor, block))

            def analysis(*_):
                loop.call_soon_threadsafe(enqueue_blocker)
                return "analysis"

            async def tracked(function, *args, **kwargs):
                if function is self.bot.log_opportunity or (function is self.bot.run_market_data_action
                        and len(args) > 2 and args[2] is self.bot.log_opportunity):
                    log_queued.set()
                return await REAL_TO_THREAD(function, *args, **kwargs)

            with patch.object(self.bot, "ask_llm_analysis", side_effect=analysis), patch.object(
                    self.bot.asyncio, "to_thread", new=tracked):
                task = asyncio.create_task(self.bot.handle_price_update("Kraken", self.symbol, 100))
                try:
                    await asyncio.wait_for(log_queued.wait(), 2)
                    await asyncio.wait_for(executor_blocked.wait(), 2)
                    self.bot.price_update_generation["Binance"][self.symbol] += 1
                    release.set()
                    await task
                finally:
                    release.set()
                    await self.stop(task)
                    await asyncio.gather(*blockers)
        self.bot.log_opportunity.assert_not_called()

    async def test_dispatcher_coalesces_1000_ticks_into_one_immutable_pending_value(self):
        entered, release, finished = asyncio.Event(), asyncio.Event(), asyncio.Event()
        processed = []

        async def process(update):
            processed.append(update)
            if len(processed) == 1:
                entered.set()
                await release.wait()
            else:
                finished.set()

        with patch.object(self.bot, "process_price_update", new=process):
            async with self.bot.feed_dispatcher_scope() as dispatcher:
                first = self.bot.record_price_update("Kraken", self.symbol, 100)
                with self.assertRaises(FrozenInstanceError):
                    first.price = 999
                dispatcher.submit(first)
                await entered.wait()
                try:
                    for index in range(1000):
                        dispatcher.submit(self.bot.record_price_update(
                            "Kraken", self.symbol, 100 + index / 10_000))
                    self.assertEqual(len(dispatcher.workers), 3)
                    self.assertEqual(len(dispatcher.pending), 1)
                    self.assertEqual(dispatcher.pending[self.symbol].price, 100.0999)
                    self.assertEqual(first.price, 100)
                finally:
                    release.set()
                await asyncio.wait_for(finished.wait(), 1)
                self.assertEqual([update.price for update in processed], [100, 100.0999])
            self.assertTrue(all(worker.done() for worker in dispatcher.workers))
            self.assertIsNone(self.bot._feed_dispatcher)

    async def test_coalesced_interruptions_reset_confirmation(self):
        for interruption in ("invalid_price", "invalid_time", "unprofitable", "anomaly", "stale", "gap"):
            with self.subTest(interruption=interruption):
                now = time.monotonic()
                for exchange, price in (("Kraken", 100), ("Binance", 101)):
                    self.bot.latest_prices[exchange][self.symbol] = price
                    self.bot.last_price_update_at[exchange][self.symbol] = now
                self.bot.alert_gate = AlertGate(3)
                self.bot.alert_gate.update(self.symbol, profitable=True, spread=1,
                                          rearm_threshold=0.3, now=now - 2.9)
                if interruption == "invalid_price":
                    self.bot.record_price_update("Binance", self.symbol, "NaN")
                elif interruption == "invalid_time":
                    self.bot.record_price_update("Binance", self.symbol, 101, "invalid")
                elif interruption == "unprofitable":
                    self.bot.record_price_update("Binance", self.symbol, 100)
                elif interruption == "anomaly":
                    self.bot.record_price_update("Binance", self.symbol, 200)
                elif interruption == "stale":
                    self.bot.record_price_update("Binance", self.symbol, 101, time.time() - 20)
                else:
                    self.bot.last_price_update_at["Binance"][self.symbol] = now - 20
                    self.bot.record_price_update("Kraken", self.symbol, 100)
                self.bot.record_price_update("Binance", self.symbol, 101)
                self.assertFalse(self.bot.alert_gate.update(
                    self.symbol, profitable=True, spread=1, rearm_threshold=0.3, now=now + 0.2))

    async def test_disconnect_invalidates_quotes_and_closes_workers(self):
        socket = FakeWebSocket()
        socket.frames.put_nowait(None)
        with patch.object(self.bot.websockets, "connect", return_value=socket):
            await self.bot.stream_kraken_tickers()
        self.assertNotIn(self.symbol, self.bot.last_price_update_at["Kraken"])
        self.assertEqual(self.bot.price_update_generation["Kraken"][self.symbol], 2)
        self.assertIsNone(self.bot._feed_dispatcher)
        self.bot.paper_simulator.update.assert_not_called()

    async def test_history_keeps_event_time_of_selected_sample(self):
        event_epoch = time.time() - 20
        await self.bot.handle_price_update("Kraken", self.symbol, 100, event_epoch)
        call = self.bot.market_store.record_tick.call_args
        self.assertEqual(call.args, (self.symbol, "Kraken", 100))
        self.assertAlmostEqual(call.kwargs["timestamp"].timestamp(), event_epoch, places=3)
        self.bot.paper_simulator.update.assert_not_called()

    async def test_old_processed_snapshot_never_rewrites_newer_received_quote(self):
        old = self.bot.record_price_update("Kraken", self.symbol, 100)
        newest = self.bot.record_price_update("Kraken", self.symbol, 100.2)
        await self.bot.process_price_update(old)
        self.assertEqual(self.bot.latest_prices["Kraken"][self.symbol], 100.2)
        self.assertEqual(self.bot.last_price_update_at["Kraken"][self.symbol], newest.kraken_updated_at)
        self.assertEqual(self.bot.price_update_generation["Kraken"][self.symbol], 3)
        self.bot.paper_simulator.update.assert_not_called()
        self.bot.notify_opportunity.assert_not_called()

    async def test_generation_change_during_orderbooks_cannot_close_existing_paper_position(self):
        entered, release = threading.Event(), threading.Event()
        estimate = self.bot.fetch_public_execution_estimate.return_value

        def books(*_, **__):
            entered.set()
            release.wait(3)
            return estimate

        with tempfile.TemporaryDirectory() as directory:
            simulator = self.bot.PaperSimulator(
                mode="fixed", starting_balance_usdt=1000, position_percent=10,
                fixed_position_usdt=100, exit_spread_percent=0.05, max_hold_seconds=60,
                estimated_fees_percent=self.bot.ESTIMATED_FEES, log_path=Path(directory) / "paper.csv")
            simulator.update(self.symbol, 100, 101, opportunity_confirmed=True)
            with patch.object(self.bot, "paper_simulator", simulator), patch.object(
                    self.bot, "fetch_public_execution_estimate", side_effect=books):
                task = asyncio.create_task(self.bot.handle_price_update("Kraken", self.symbol, 100))
                try:
                    await self.wait_thread(entered)
                    simulator.positions[self.symbol] = replace(
                        simulator.positions[self.symbol], opened_at=datetime.now(timezone.utc) - timedelta(seconds=120))
                    self.bot.record_price_update("Binance", self.symbol, 100.1)
                    release.set()
                    await task
                finally:
                    release.set()
                    await self.stop(task)
            self.assertIn(self.symbol, simulator.positions)
            self.assertEqual(simulator.completed_positions, 0)
            self.assertEqual(self.bot.opportunities_count, 0)

    async def test_valid_price_movement_still_reaches_notifier(self):
        self.bot.last_movement_notification_at[self.symbol] = 0
        await self.bot.handle_price_update("Kraken", self.symbol, 100.2)
        self.bot.notify_price_movement.assert_called_once()

    async def test_snapshot_publisher_runs_while_all_three_workers_wait_for_llm(self):
        loop = asyncio.get_running_loop()
        entered = {symbol: threading.Event() for symbol in self.bot.SYMBOLS}
        release = threading.Event()
        snapshots = asyncio.Queue()
        sockets = {exchange: FakeWebSocket() for exchange in ("Kraken", "Binance")}
        now = time.monotonic()
        for symbol in self.bot.SYMBOLS:
            for exchange, price in (("Kraken", 100), ("Binance", 101)):
                self.bot.latest_prices[exchange][symbol] = price
                self.bot.last_price_update_at[exchange][symbol] = now
                self.bot.price_update_generation[exchange][symbol] = 1
            self.bot.last_movement_notification_at[symbol] = now

        def llm(symbol, *_):
            entered[symbol].set()
            release.wait(4)
            return "analysis"

        def writer(prices, timestamps, **kwargs):
            loop.call_soon_threadsafe(snapshots.put_nowait, (prices, timestamps, kwargs))

        def connect(url, **_):
            return sockets["Kraken" if "kraken" in url else "Binance"]

        async def one_session(_name, stream):
            await stream()

        with patch.object(self.bot.websockets, "connect", side_effect=connect), patch.object(
                self.bot, "ask_llm_analysis", side_effect=llm), patch.object(
                self.bot, "write_market_snapshot", side_effect=writer), patch.object(
                self.bot, "run_stream_with_reconnect", new=one_session), patch("builtins.print"):
            task = asyncio.create_task(self.bot.monitor_markets())
            try:
                for symbol in self.bot.SYMBOLS:
                    sockets["Kraken"].push("Kraken", symbol, 100)
                for event in entered.values():
                    await self.wait_thread(event)
                while not snapshots.empty():
                    snapshots.get_nowait()
                epoch = time.time() - 0.25
                for symbol in self.bot.SYMBOLS:
                    sockets["Kraken"].push("Kraken", symbol, 100.25, epoch)
                    sockets["Binance"].push("Binance", symbol, 101.25, epoch)
                while True:
                    prices, timestamps, captured = await asyncio.wait_for(snapshots.get(), 1.5)
                    if all(prices["Kraken"].get(symbol) == 100.25
                           and prices["Binance"].get(symbol) == 101.25 for symbol in self.bot.SYMBOLS):
                        break
                self.assertFalse(release.is_set())
                self.assertIsNot(prices, self.bot.latest_prices)
                self.assertIsNot(timestamps["Kraken"], self.bot.last_price_update_at["Kraken"])
                self.assertAlmostEqual(captured["now_monotonic"] - timestamps["Kraken"][self.symbol],
                                       captured["now_utc"].timestamp() - epoch, places=2)
            finally:
                release.set()
                await self.stop(task)

    async def test_periodic_snapshot_writer_has_one_write_in_flight(self):
        loop = asyncio.get_running_loop()
        entered, release = threading.Event(), threading.Event()
        second_write = asyncio.Event()
        snapshots = []

        def writer(prices, timestamps, **kwargs):
            snapshots.append((prices, timestamps, kwargs))
            if len(snapshots) == 1:
                entered.set()
                release.wait(3)
            elif len(snapshots) == 2:
                loop.call_soon_threadsafe(second_write.set)

        with patch.object(self.bot, "write_market_snapshot", side_effect=writer), patch.object(
                self.bot, "MARKET_SNAPSHOT_INTERVAL_SECONDS", 0.01):
            task = asyncio.create_task(self.bot.publish_market_snapshots_periodically())
            try:
                await self.wait_thread(entered)
                for index in range(1000):
                    self.bot.record_price_update("Kraken", self.symbol, 100 + index / 10_000)
                await asyncio.sleep(0.05)
                self.assertEqual(len(snapshots), 1)
                self.assertEqual(snapshots[0][0]["Kraken"][self.symbol], 100)
                release.set()
                await asyncio.wait_for(second_write.wait(), 1)
                self.assertEqual(snapshots[1][0]["Kraken"][self.symbol], 100.0999)
            finally:
                release.set()
                await self.stop(task)


if __name__ == "__main__":
    unittest.main()
