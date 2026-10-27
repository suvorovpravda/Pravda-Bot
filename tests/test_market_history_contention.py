from concurrent.futures import ThreadPoolExecutor
import sqlite3
import tempfile
import threading
import unittest
from datetime import datetime, timezone
from pathlib import Path
from unittest.mock import Mock, patch

from market_history import MarketStore, WRITE_ATTEMPTS


class MarketStoreContentionTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.path = Path(directory.name) / 'history.sqlite3'
        self.store = MarketStore(self.path)
        self.now = datetime(2026, 10, 27, 12, tzinfo=timezone.utc)

    def tearDown(self):
        # Force garbage collection to close lingering SQLite connections on Windows
        import gc
        gc.collect()

    def count(self):
        with sqlite3.connect(self.path) as connection:
            return connection.execute('SELECT COUNT(*) FROM market_ticks').fetchone()[0]

    def test_wal_readers_continue_and_writer_retries_after_real_lock_is_released(self):
        with sqlite3.connect(self.path) as blocker:
            self.assertEqual(blocker.execute('PRAGMA journal_mode').fetchone()[0], 'wal')
            blocker.execute('BEGIN IMMEDIATE')
            retry_entered, released = threading.Event(), threading.Event()

            def wait_for_release(_delay):
                retry_entered.set()
                if not released.wait(2):
                    raise AssertionError('lock was not released')

            with ThreadPoolExecutor(max_workers=1) as executor, patch(
                'market_history.WRITE_BUSY_TIMEOUT_MS', 0
            ), patch('market_history.time.sleep', side_effect=wait_for_release) as sleep:
                future = executor.submit(
                    self.store.record_tick, 'BTC/USDT', 'Kraken', 100, timestamp=self.now
                )
                try:
                    self.assertTrue(retry_entered.wait(2), 'writer did not encounter the lock')
                    self.assertEqual(self.store.history('BTC/USDT', 'Kraken', 'day', now=self.now), [])
                finally:
                    blocker.commit()
                    released.set()
                future.result(timeout=2)
                sleep.assert_called_once_with(0.025)
        self.assertEqual(self.count(), 1)

    def test_persistent_real_lock_fails_after_bounded_attempts_without_insert(self):
        with sqlite3.connect(self.path) as blocker:
            blocker.execute('BEGIN IMMEDIATE')
            with patch('market_history.WRITE_BUSY_TIMEOUT_MS', 0), patch(
                'market_history.time.sleep'
            ) as sleep, self.assertRaises(sqlite3.OperationalError) as failure:
                self.store.record_tick('BTC/USDT', 'Kraken', 100, timestamp=self.now)
            self.assertEqual(failure.exception.sqlite_errorcode & 0xff, sqlite3.SQLITE_BUSY)
            self.assertEqual(sleep.call_count, WRITE_ATTEMPTS - 1)
            self.assertEqual([c.args[0] for c in sleep.call_args_list], [0.025, 0.05, 0.1, 0.2])
        self.assertEqual(self.count(), 0)

    def test_non_contention_sql_error_is_never_retried(self):
        def invalid_sql(connection):
            connection.execute('INSERT INTO missing_table VALUES (1)')
        operation = Mock(side_effect=invalid_sql)
        with patch('market_history.time.sleep') as sleep, self.assertRaises(sqlite3.OperationalError):
            self.store._write_with_retry(operation)
        operation.assert_called_once()
        sleep.assert_not_called()

    def test_retry_rolls_back_entire_partial_transaction_before_replay(self):
        calls = 0
        def write(connection):
            nonlocal calls
            calls += 1
            connection.execute(
                'INSERT INTO market_ticks(timestamp_utc, symbol, exchange, price) VALUES (?, ?, ?, ?)',
                ('2026-10-27T12:00:00.000Z', 'BTC/USDT', 'Kraken', 100),
            )
            if calls == 1:
                failure = sqlite3.OperationalError('transient contention')
                failure.sqlite_errorcode = sqlite3.SQLITE_BUSY
                raise failure
            return 'committed'
        with patch('market_history.time.sleep'):
            self.assertEqual(self.store._write_with_retry(write), 'committed')
        self.assertEqual(calls, 2)
        self.assertEqual(self.count(), 1)

    def test_independent_stores_write_concurrently_without_missing_ticks(self):
        stores = [MarketStore(self.path) for _ in range(8)]
        def write(index):
            stores[index % len(stores)].record_tick('BTC/USDT', 'Kraken', index + 1, timestamp=self.now)
        with ThreadPoolExecutor(max_workers=8) as executor:
            list(executor.map(write, range(100)))
        self.assertEqual(self.count(), 100)

    def test_concurrent_alert_creation_never_exceeds_limit(self):
        stores = [MarketStore(self.path) for _ in range(8)]
        def create(index):
            try:
                return stores[index % 8].create_alert('BTC/USDT', 'Kraken', 'EUR', 'above', 100)
            except ValueError as error:
                self.assertIn('at most 50', str(error))
                return None
        with ThreadPoolExecutor(max_workers=8) as executor:
            results = list(executor.map(create, range(60)))
        self.assertEqual(sum(result is not None for result in results), 50)
        self.assertEqual(len(self.store.list_alerts()), 50)

    def test_invalid_numeric_inputs_never_trigger_alerts_or_insert_ticks(self):
        self.store.create_alert('BTC/USDT', 'Kraken', 'CHF', 'above', 1)
        for bad in (True, float('nan'), float('inf'), -1, 0):
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                self.store.record_tick('BTC/USDT', 'Kraken', bad)
            with self.assertRaises(ValueError):
                self.store.create_alert('BTC/USDT', 'Kraken', 'EUR', 'above', bad)
            for price, usd, chf in ((bad, 1, 1), (100, bad, 1), (100, 1, bad), (1e308, 1e-308, 1)):
                snapshot = {'symbols': {'BTC/USDT': {'Kraken': {'price': price, 'status': 'live'}}}}
                self.assertEqual(self.store.check_alerts(snapshot, {'rates': {'USD': usd, 'CHF': chf}}), [])
        self.assertEqual(self.count(), 0)
