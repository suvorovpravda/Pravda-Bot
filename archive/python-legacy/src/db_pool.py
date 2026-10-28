# SQLite connection pool
import sqlite3
from queue import Queue
from threading import Lock

class SQLitePool:
    """Simple thread‑safe pool for SQLite connections.
    Each connection is configured with `check_same_thread=False` so that it can be used from any thread.
    """

    def __init__(self, db_path: str, size: int = 5):
        self.db_path = db_path
        self.size = size
        self._pool = Queue(maxsize=size)
        for _ in range(size):
            conn = sqlite3.connect(self.db_path, timeout=5, check_same_thread=False)
            conn.row_factory = sqlite3.Row
            # SQLite busy_timeout is set via pragma; keep default 5000ms as before
            self._pool.put(conn)
        self._lock = Lock()

    def acquire(self) -> sqlite3.Connection:
        return self._pool.get()

    def release(self, conn: sqlite3.Connection):
        self._pool.put(conn)

    def closeall(self):
        while not self._pool.empty():
            conn = self._pool.get()
            conn.close()
