class AlertGate:
    def __init__(self, confirmation_seconds: float) -> None:
        if confirmation_seconds < 0:
            raise ValueError("confirmation_seconds must be non-negative")
        self.confirmation_seconds = confirmation_seconds
        self._candidate_since = {}
        self._active = set()
        self._activation_ids = {}
        self._next_activation_id = 0

    def reset_candidate(self, symbol: str) -> None:
        self._candidate_since.pop(symbol, None)

    def activation_id(self, symbol: str) -> int | None:
        return self._activation_ids.get(symbol) if symbol in self._active else None

    def cancel_activation(self, symbol: str, activation_id: int) -> bool:
        if self.activation_id(symbol) != activation_id:
            return False
        self._active.discard(symbol)
        self._activation_ids.pop(symbol, None)
        return True

    def is_active(self, symbol: str) -> bool:
        return symbol in self._active

    def update(
        self,
        symbol: str,
        *,
        profitable: bool,
        spread: float,
        rearm_threshold: float,
        now: float,
    ) -> bool:
        if symbol in self._active:
            if spread <= rearm_threshold:
                self._active.remove(symbol)
                self._activation_ids.pop(symbol, None)
            else:
                return False

        if not profitable:
            self.reset_candidate(symbol)
            return False

        candidate_since = self._candidate_since.setdefault(symbol, now)
        if now - candidate_since + 1e-9 < self.confirmation_seconds:
            return False

        self._candidate_since.pop(symbol, None)
        self._active.add(symbol)
        self._next_activation_id += 1
        self._activation_ids[symbol] = self._next_activation_id
        return True
