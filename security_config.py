"""Explicit outbound HTTP policies; importing this module performs no I/O."""
from __future__ import annotations

import math
from dataclasses import dataclass
from types import MappingProxyType


MAX_RESPONSE_BYTES = 10 * 1024 * 1024
MAX_RETRY_ATTEMPTS = 3
HTTP_TIMEOUT_SECONDS = 10.0
CONNECT_TIMEOUT_SECONDS = 4.0
RETRY_BASE_DELAY_SECONDS = 0.25
RETRY_MAX_DELAY_SECONDS = 1.0
STREAM_CHUNK_BYTES = 64 * 1024
LOCAL_LM_HOSTS = frozenset({"127.0.0.1", "localhost", "::1"})
LOCAL_LM_PORT = 1234

# Exact host sets for the maintained adapters, never suffix matches.
SERVICE_HOSTS = MappingProxyType({
    "entsog": frozenset({"transparency.entsog.eu"}),
    "gdelt": frozenset({"api.gdeltproject.org"}),
    "bbc": frozenset({"feeds.bbci.co.uk"}),
    "fx": frozenset({"api.frankfurter.dev"}),
    "ntfy": frozenset({"ntfy.sh"}),
    "lm_studio": LOCAL_LM_HOSTS,
})


class SecurityValidationError(ValueError):
    """Invalid policy; error messages contain no caller-supplied values."""


@dataclass(frozen=True, slots=True)
class SecurityConfig:
    """Bounded HTTP settings. Requests timeouts are inactivity timeouts.

    The client checks an elapsed budget between body chunks, but OS DNS calls
    and a server trickling response headers are not forcibly cancellable here.
    """

    timeout: float = HTTP_TIMEOUT_SECONDS
    max_bytes: int = MAX_RESPONSE_BYTES
    attempts: int = 2

    def __post_init__(self) -> None:
        if (isinstance(self.timeout, bool) or not isinstance(self.timeout, (int, float))
                or not math.isfinite(self.timeout) or not 0 < self.timeout <= 60):
            raise SecurityValidationError("HTTP timeout must be finite and between 0 and 60 seconds")
        if (isinstance(self.max_bytes, bool) or not isinstance(self.max_bytes, int)
                or not 1 <= self.max_bytes <= MAX_RESPONSE_BYTES):
            raise SecurityValidationError("HTTP response limit must be between 1 byte and 10 MiB")
        if (isinstance(self.attempts, bool) or not isinstance(self.attempts, int)
                or not 1 <= self.attempts <= MAX_RETRY_ATTEMPTS):
            raise SecurityValidationError("HTTP attempts must be between 1 and 3")
