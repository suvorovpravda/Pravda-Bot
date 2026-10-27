"""Standalone Kraken v2 utility. Timing measures local work, not network transit.
The active bot uses the readers in root main.py.
"""
from __future__ import annotations
import inspect
import json
import time
import websockets


class KrakenWebSocket:
    def __init__(self, url="wss://ws.kraken.com/v2"):
        if url != "wss://ws.kraken.com/v2":
            raise ValueError("Only the public Kraken v2 endpoint is supported")
        self.url = url
        self._callbacks = []
        self._last_tick_time_ms = None

    def on_message(self, callback):
        self._callbacks.append(callback)

    async def connect(self):
        async with websockets.connect(self.url, open_timeout=15, ping_interval=20, ping_timeout=20) as stream:
            await stream.send(json.dumps({"method": "subscribe", "params": {
                "channel": "ticker", "symbol": ["BTC/USDT", "ETH/USDT", "SOL/USDT"]}}))
            async for raw in stream:
                await self.process_kraken_message(raw)

    async def process_kraken_message(self, raw_message):
        started = time.monotonic()
        data = json.loads(raw_message)
        decoded = time.monotonic()
        if isinstance(data, dict) and data.get("channel") == "ticker":
            for tick in data.get("data", []):
                if not isinstance(tick, dict) or "last" not in tick:
                    continue
                for callback in self._callbacks:
                    result = callback(tick)
                    if inspect.isawaitable(result):
                        await result
                self._last_tick_time_ms = decoded * 1000
        finished = time.monotonic()
        return {"decode_duration_ms": (decoded-started)*1000,
                "process_duration_ms": (finished-decoded)*1000,
                "total_duration_ms": (finished-started)*1000}


async def process_kraken_message_with_timing(raw_message):
    return await KrakenWebSocket().process_kraken_message(raw_message)
