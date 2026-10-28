import asyncio

class BoundedFeedQueue:
    """Bounded queue for incoming ticks.
    When full, the oldest tick is dropped to make room for new data.
    This guarantees that the consumer always receives the most recent information
    and prevents memory bloat under high load.
    """

    def __init__(self, maxsize: int = 100):
        self._queue = asyncio.Queue(maxsize=maxsize)

    async def put_tick(self, tick: dict):
        if self._queue.full():
            try:
                # discard the oldest tick to free space for the new one
                self._queue.get_nowait()
            except asyncio.QueueEmpty:
                pass
        await self._queue.put(tick)

    async def get_tick(self):
        return await self._queue.get()
