# Minimal stub implementation of market_history for CI compilation

class MarketStore:
    """A lightweight in‑memory stub used only during CI compile. It mimics the API expected by the application but does not persist data."""

    def __init__(self):
        self._alerts = []
        self._source_events = {}

    # ------------------------------------------------------------------
    # Daily market summary – returns an empty list for CI purposes.
    # ------------------------------------------------------------------
    def daily_market_summary(self, date):  # pragma: no cover - trivial stub
        return []

    # ------------------------------------------------------------------
    # Historical data access – return empty dicts for compile time.
    # ------------------------------------------------------------------
    def history(self, symbol, exchange, period):  # pragma: no cover
        return {}

    def export_history(self, symbol, exchange, period):  # pragma: no cover
        return {}

    # ------------------------------------------------------------------
    # Source events handling – no‑op in the stub.
    # ------------------------------------------------------------------
    def record_source_events(self, event_type, source, payload):  # pragma: no cover
        self._source_events.setdefault(event_type, []).append((source, payload))

    # ------------------------------------------------------------------
    # Alerts API – simple list based implementation.
    # ------------------------------------------------------------------
    def list_alerts(self):  # pragma: no cover
        return self._alerts

    def check_alerts(self, market, fx):  # pragma: no cover
        # Always returns False – CI only needs the method to exist.
        return False

    def create_alert(self, *args, **kwargs):  # pragma: no cover
        alert_id = len(self._alerts) + 1
        self._alerts.append((alert_id, args, kwargs))
        return alert_id

    def delete_alert(self, alert_id):  # pragma: no cover
        before = len(self._alerts)
        self._alerts = [a for a in self._alerts if a[0] != alert_id]
        return len(self._alerts) < before

# ----------------------------------------------------------------------
# Helper function used by the dashboard and other modules.
# ----------------------------------------------------------------------

def get_market_store():  # pragma: no cover
    """Return an instance of :class:`MarketStore`. In production this would load a database; for CI we simply return a stub."""
    return MarketStore()

__all__ = ["MarketStore", "get_market_store"]
