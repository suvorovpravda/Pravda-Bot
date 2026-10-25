class AlertGate:
    def __init__(self, confirmation_seconds: float) -> None:
        if confirmation_seconds < 0:
            raise ValueError("confirmation_seconds must be non-negative")
        self.confirmation_seconds = confirmation_seconds
        self._candidate_since = {}
        self._active = set()

    def reset_candidate(self, symbol: str) -> None:
        self._candidate_since.pop(symbol, None)

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
        return True
