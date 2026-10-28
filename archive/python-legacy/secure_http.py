"""Synchronous HTTP with exact destinations and bounded streaming reads.

Use worker threads, never the async event loop, for this client. DNS is checked
before each attempt; requests resolves again on connection. This is defense in
depth, not DNS/IP pinning or an egress firewall. Errors never interpolate URLs,
queries, headers, response bodies, credentials or notification topics.
"""
from __future__ import annotations

import ipaddress
import json as json_module
import random
import re
import socket
import time
from dataclasses import dataclass
from types import MappingProxyType
from typing import Any, Collection, Mapping
from urllib.parse import SplitResult, urlsplit

import requests
from requests.structures import CaseInsensitiveDict

from security_config import (
    CONNECT_TIMEOUT_SECONDS, HTTP_TIMEOUT_SECONDS, LOCAL_LM_HOSTS,
    LOCAL_LM_PORT, MAX_RESPONSE_BYTES, RETRY_BASE_DELAY_SECONDS,
    RETRY_MAX_DELAY_SECONDS, STREAM_CHUNK_BYTES, SecurityConfig,
    SecurityValidationError,
)


class SecureHTTPError(RuntimeError):
    """Sanitized error, suitable for the application's ordinary error logs."""


@dataclass(frozen=True, slots=True)
class SecureHTTPResponse:
    status_code: int
    headers: Mapping[str, str]
    content: bytes

    @property
    def text(self) -> str:
        return self.content.decode("utf-8", errors="replace")

    def json(self) -> Any:
        def reject_constant(_value: str) -> None:
            raise ValueError("Non-finite JSON constant")

        try:
            # Preserve JSON lists: Frankfurter returns one.
            return json_module.loads(self.content, parse_constant=reject_constant)
        except (ValueError, UnicodeError, RecursionError):
            raise SecureHTTPError("Invalid JSON response") from None

    def raise_for_status(self) -> None:
        if not 200 <= self.status_code < 300:
            raise SecureHTTPError(f"HTTP status {self.status_code}")


def _canonical_host(host: str) -> str:
    if not isinstance(host, str) or not host or not host.isascii() or "%" in host:
        raise SecureHTTPError("Invalid allowed HTTP host")
    host = host.lower()
    try:
        return str(ipaddress.ip_address(host))
    except ValueError:
        if not re.fullmatch(
            r"(?=.{1,253}\Z)[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?"
            r"(?:\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)*", host
        ):
            raise SecureHTTPError("Invalid allowed HTTP host") from None
        return host


def validate_url(
    url: str, *, allowed_hosts: Collection[str], local_only: bool = False,
    allowed_ports: Collection[int] | None = None,
) -> SplitResult:
    """Validate without DNS. local_only is the explicit HTTP LM Studio exception."""
    if (not isinstance(url, str) or not url or len(url) > 8192
            or "\\" in url or "#" in url
            or any(c.isspace() or ord(c) < 32 or ord(c) == 127 for c in url)):
        raise SecureHTTPError("Invalid HTTP URL")
    if not isinstance(local_only, bool):
        raise SecureHTTPError("Invalid local HTTP policy")
    if isinstance(allowed_hosts, str) or not allowed_hosts or len(allowed_hosts) > 64:
        raise SecureHTTPError("An exact HTTP host allowlist is required")
    hosts = frozenset(_canonical_host(host) for host in allowed_hosts)
    ports = frozenset(allowed_ports if allowed_ports is not None else
                      {LOCAL_LM_PORT} if local_only else {443})
    if not ports or any(isinstance(p, bool) or not isinstance(p, int) or not 1 <= p <= 65535
                        for p in ports):
        raise SecureHTTPError("Invalid allowed HTTP ports")
    try:
        parsed = urlsplit(url)
        host = _canonical_host(parsed.hostname or "")
        port = parsed.port if parsed.port is not None else (80 if parsed.scheme == "http" else 443)
    except (TypeError, ValueError):
        raise SecureHTTPError("Invalid HTTP URL") from None
    if (parsed.username is not None or parsed.password is not None
            or host not in hosts or port not in ports):
        raise SecureHTTPError("HTTP destination is not allowed")
    if local_only:
        if parsed.scheme != "http" or host not in LOCAL_LM_HOSTS:
            raise SecureHTTPError("Local HTTP requires an approved loopback destination")
    elif parsed.scheme != "https" or host in LOCAL_LM_HOSTS:
        raise SecureHTTPError("Public HTTP requires HTTPS")
    return parsed


def _check_address(address: str, *, local_only: bool) -> None:
    try:
        ip = ipaddress.ip_address(address)
    except ValueError:
        raise SecureHTTPError("HTTP DNS returned an invalid address") from None
    if isinstance(ip, ipaddress.IPv6Address) and ip.ipv4_mapped is not None:
        ip = ip.ipv4_mapped
    if local_only:
        allowed = ip.is_loopback
    else:
        allowed = (ip.is_global and not ip.is_multicast and not ip.is_unspecified
                   and not ip.is_reserved and not ip.is_loopback and not ip.is_link_local)
        if isinstance(ip, ipaddress.IPv6Address) and (ip.sixtofour is not None or ip.teredo is not None):
            allowed = False
    if not allowed:
        raise SecureHTTPError("HTTP destination address is not allowed")


def _check_dns(parsed: SplitResult, *, local_only: bool) -> None:
    port = parsed.port if parsed.port is not None else (80 if parsed.scheme == "http" else 443)
    try:
        literal = ipaddress.ip_address(parsed.hostname or "")
    except ValueError:
        try:
            answers = socket.getaddrinfo(parsed.hostname, port, type=socket.SOCK_STREAM)
        except OSError:
            raise SecureHTTPError("HTTP DNS resolution failed") from None
        if not answers:
            raise SecureHTTPError("HTTP DNS returned no addresses")
        for answer in answers:
            _check_address(answer[4][0], local_only=local_only)
    else:
        _check_address(str(literal), local_only=local_only)


def _read_body(response: requests.Response, *, max_bytes: int, deadline: float) -> bytes:
    length = response.headers.get("Content-Length")
    if length is not None:
        try:
            if not 0 <= int(length) <= max_bytes:
                raise ValueError
        except (ValueError, TypeError):
            raise SecureHTTPError("HTTP response exceeds the size limit") from None
    body = bytearray()
    for chunk in response.iter_content(chunk_size=min(STREAM_CHUNK_BYTES, max_bytes + 1)):
        if time.monotonic() > deadline:
            raise SecureHTTPError("HTTP response exceeded its time budget")
        if not chunk:
            continue
        # Requests yields decompressed bytes; compressed length does not evade the cap.
        if len(chunk) > max_bytes - len(body):
            raise SecureHTTPError("HTTP response exceeds the size limit")
        body.extend(chunk)
    if time.monotonic() > deadline:
        raise SecureHTTPError("HTTP response exceeded its time budget")
    return bytes(body)


def request(
    url: str, *, method: str = "GET", allowed_hosts: Collection[str],
    local_only: bool = False, allowed_ports: Collection[int] | None = None,
    params: Mapping[str, Any] | None = None, json: Any = None,
    data: str | bytes | None = None, headers: Mapping[str, str] | None = None,
    timeout: float = HTTP_TIMEOUT_SECONDS, max_bytes: int = MAX_RESPONSE_BYTES,
    attempts: int = 2,
) -> SecureHTTPResponse:
    """Return even a final 429/5xx with headers; HTTP errors are caller policy.

    GET alone retries connection/read failures and 408/429/502/503/504. POST is
    never replayed. Redirects fail. Timeouts limit transport inactivity; DNS and
    trickled headers are not a guaranteed wall-clock deadline.
    """
    try:
        limits = SecurityConfig(timeout=timeout, max_bytes=max_bytes, attempts=attempts)
    except SecurityValidationError:
        raise SecureHTTPError("Invalid HTTP request limits") from None
    if not isinstance(method, str) or method.upper() not in {"GET", "POST"}:
        raise SecureHTTPError("HTTP method is not allowed")
    method = method.upper()
    if json is not None and data is not None:
        raise SecureHTTPError("HTTP request cannot contain two bodies")
    if data is not None and not isinstance(data, (str, bytes)):
        raise SecureHTTPError("HTTP request body must be text or bytes")
    request_headers = {"User-Agent": "Pravda-Bot/1.0", "Accept-Encoding": "identity"}
    if headers:
        if any(str(k).lower() in {"host", "proxy-authorization", "cookie"} for k in headers):
            raise SecureHTTPError("HTTP header override is not allowed")
        request_headers.update(headers)
    parsed = validate_url(url, allowed_hosts=allowed_hosts, local_only=local_only,
                          allowed_ports=allowed_ports)
    count = limits.attempts if method == "GET" else 1
    deadline = time.monotonic() + limits.timeout * count + RETRY_MAX_DELAY_SECONDS * (count - 1)
    for attempt in range(count):
        _check_dns(parsed, local_only=local_only)
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise SecureHTTPError("HTTP request exceeded its time budget")
        retry = False
        try:
            # A session per attempt avoids mutable sessions/cookies shared across worker threads.
            with requests.Session() as session:
                session.trust_env = False  # No proxy env, netrc credentials or CA override.
                with session.request(
                    method, url, params=params, json=json, data=data, headers=request_headers,
                    stream=True, allow_redirects=False, verify=True,
                    timeout=(min(CONNECT_TIMEOUT_SECONDS, remaining, limits.timeout),
                             min(remaining, limits.timeout)),
                ) as response:
                    status = response.status_code
                    if isinstance(status, bool) or not isinstance(status, int) or not 100 <= status <= 599:
                        raise SecureHTTPError("Invalid HTTP response status")
                    if 300 <= status < 400:
                        raise SecureHTTPError("HTTP redirects are not allowed")
                    if method == "GET" and status in {408, 429, 502, 503, 504} and attempt + 1 < count:
                        retry = True
                    else:
                        response_headers = CaseInsensitiveDict(response.headers)
                        if sum(len(str(k)) + len(str(v)) for k, v in response_headers.items()) > 64 * 1024:
                            raise SecureHTTPError("HTTP response headers exceed the limit")
                        body = _read_body(response, max_bytes=limits.max_bytes, deadline=deadline)
                        return SecureHTTPResponse(status, MappingProxyType(response_headers), body)
        except (requests.Timeout, requests.ConnectionError):
            retry = attempt + 1 < count
        except requests.RequestException:
            raise SecureHTTPError("HTTP request failed") from None
        if not retry:
            raise SecureHTTPError("HTTP request failed") from None
        cap = min(RETRY_MAX_DELAY_SECONDS, RETRY_BASE_DELAY_SECONDS * (2 ** attempt))
        time.sleep(random.SystemRandom().uniform(0, cap))
    raise SecureHTTPError("HTTP request failed")


def get_json(url: str, **kwargs: Any) -> Any:
    response = request(url, **kwargs)
    response.raise_for_status()
    return response.json()


def get_text(url: str, **kwargs: Any) -> str:
    response = request(url, **kwargs)
    response.raise_for_status()
    return response.text
