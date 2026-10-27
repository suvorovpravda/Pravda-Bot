import ipaddress
import socket
import unittest
from collections import deque
from contextlib import contextmanager
from unittest.mock import Mock, patch

import requests
from requests.structures import CaseInsensitiveDict

from secure_http import SecureHTTPError, SecureHTTPResponse, get_json, request, validate_url
from security_config import LOCAL_LM_HOSTS, MAX_RESPONSE_BYTES, SERVICE_HOSTS, SecurityConfig


URL = "https://api.gdeltproject.org/api/v2/doc/doc"
HOSTS = SERVICE_HOSTS["gdelt"]


class ResponseFixture:
    def __init__(self, chunks=(b'{"ok":true}',), *, status=200, headers=None):
        self.status_code = status
        self.headers = CaseInsensitiveDict(headers or {})
        self.chunks, self.chunks_read, self.closed = chunks, 0, False

    @property
    def content(self):
        raise AssertionError("Transport response must never be buffered through .content")

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.closed = True

    def iter_content(self, *, chunk_size):
        for chunk in self.chunks:
            self.chunks_read += 1
            yield chunk


@contextmanager
def transport(*responses, addresses=("8.8.8.8",)):
    pending, sessions = deque(responses), []

    def new_session():
        session = Mock()
        sessions.append(session)
        session.__enter__ = Mock(return_value=session)
        session.__exit__ = Mock(return_value=False)

        def send(*args, **kwargs):
            value = pending.popleft()
            if isinstance(value, BaseException):
                raise value
            return value

        session.request.side_effect = send
        return session

    answers = [(socket.AF_INET6 if ipaddress.ip_address(ip).version == 6 else socket.AF_INET,
                socket.SOCK_STREAM, socket.IPPROTO_TCP, "", (ip, 443)) for ip in addresses]
    with patch("secure_http.requests.Session", side_effect=new_session) as factory, patch(
        "secure_http.socket.getaddrinfo", return_value=answers
    ) as resolver, patch("secure_http.time.sleep") as sleep:
        yield sessions, factory, resolver, sleep


class SecureHTTPTests(unittest.TestCase):
    def test_exact_service_hosts_and_config_validation_do_not_resolve_dns(self):
        with patch("secure_http.socket.getaddrinfo") as dns:
            self.assertEqual(validate_url(URL, allowed_hosts=HOSTS).hostname, "api.gdeltproject.org")
            dns.assert_not_called()
        self.assertEqual(SERVICE_HOSTS["entsog"], frozenset({"transparency.entsog.eu"}))
        self.assertEqual(SERVICE_HOSTS["fx"], frozenset({"api.frankfurter.dev"}))

    def test_url_confusion_fails_before_network(self):
        urls = (
            "https://api.gdeltproject.org:443@evil.invalid/private-topic",
            "https://user:private-token@api.gdeltproject.org/",
            "https://api.gdeltproject.org.evil.invalid/",
            "https://sub.api.gdeltproject.org/", "https://feeds.bbci.co.uk/",
            "https://api.gdeltproject.org/#private-token", "https://api.gdeltproject.org/#",
            "https://api.gdeltproject.org:0/", "https://api.gdeltproject.org:444/",
            "http://api.gdeltproject.org/", "file://api.gdeltproject.org/private-token",
            "https://api.gdeltproject.org\\@evil.invalid/", " https://api.gdeltproject.org/",
            "https://api.gdeltproject.org/\r\nprivate-token", "https://аpi.gdeltproject.org/",
            "https://api.gdeltproject.org./",
        )
        with patch("secure_http.requests.Session") as factory, patch("secure_http.socket.getaddrinfo") as dns:
            for url in urls:
                with self.subTest(url=url), self.assertRaises(SecureHTTPError) as error:
                    request(url, allowed_hosts=HOSTS)
                self.assertNotIn("private-", str(error.exception))
            factory.assert_not_called()
            dns.assert_not_called()

    def test_private_metadata_reserved_multicast_and_mapped_dns_answers_fail(self):
        addresses = ("127.0.0.1", "10.0.0.1", "172.16.0.1", "192.168.0.1", "169.254.169.254",
                     "0.0.0.0", "100.64.0.1", "198.51.100.1", "224.0.0.1", "::1", "::",
                     "fe80::1", "fc00::1", "::ffff:127.0.0.1", "2002:0808:0808::1")
        for address in addresses:
            with self.subTest(address=address), transport(addresses=(address,)) as (_, factory, _, _):
                with self.assertRaises(SecureHTTPError):
                    request(URL, allowed_hosts=HOSTS)
                factory.assert_not_called()

    def test_every_dns_answer_must_pass(self):
        with transport(addresses=("8.8.8.8", "127.0.0.1")) as (_, factory, _, _):
            with self.assertRaises(SecureHTTPError):
                request(URL, allowed_hosts=HOSTS)
            factory.assert_not_called()

    def test_public_ip_literal_and_ipv6_are_supported(self):
        with transport(ResponseFixture(), addresses=("2001:4860:4860::8888",)):
            self.assertEqual(request(URL, allowed_hosts=HOSTS).status_code, 200)
        with transport(ResponseFixture()) as (_, _, dns, _):
            self.assertEqual(request("https://8.8.8.8/", allowed_hosts={"8.8.8.8"}).status_code, 200)
            dns.assert_not_called()

    def test_numeric_private_aliases_cannot_bypass_dns(self):
        for host in ("2130706433", "0x7f000001", "127.1"):
            with self.subTest(host=host), transport(addresses=("127.0.0.1",)) as (_, factory, _, _):
                with self.assertRaises(SecureHTTPError):
                    request(f"https://{host}/", allowed_hosts={host})
                factory.assert_not_called()

    def test_local_lm_exception_is_explicit_and_port_scoped(self):
        for url in ("http://127.0.0.1:1234/v1", "http://localhost:1234/v1", "http://[::1]:1234/v1"):
            with self.subTest(url=url), transport(ResponseFixture(), addresses=("127.0.0.1", "::1")):
                result = request(url, method="POST", allowed_hosts=LOCAL_LM_HOSTS, local_only=True)
                self.assertEqual(result.status_code, 200)
        with transport(ResponseFixture(), addresses=("127.0.0.1",)):
            self.assertEqual(request("http://localhost:4321/v1", allowed_hosts=LOCAL_LM_HOSTS,
                                     local_only=True, allowed_ports={4321}).status_code, 200)
        for url, hosts, local in (
            ("http://localhost:4321/v1", LOCAL_LM_HOSTS, True),
            ("http://localhost:1234/v1", LOCAL_LM_HOSTS, False),
            ("https://localhost:1234/v1", LOCAL_LM_HOSTS, True),
            ("http://192.168.1.2:1234/v1", {"192.168.1.2"}, True), (URL, HOSTS, True),
        ):
            with self.subTest(url=url, local=local), self.assertRaises(SecureHTTPError):
                request(url, allowed_hosts=hosts, local_only=local)

    def test_localhost_dns_must_stay_loopback(self):
        with transport(addresses=("8.8.8.8",)) as (_, factory, _, _):
            with self.assertRaises(SecureHTTPError):
                request("http://localhost:1234/v1", allowed_hosts=LOCAL_LM_HOSTS, local_only=True)
            factory.assert_not_called()

    def test_transport_has_tls_timeouts_streaming_and_no_environment_auth(self):
        fixture = ResponseFixture()
        with transport(fixture) as (sessions, _, _, _):
            result = request(URL, allowed_hosts=HOSTS, timeout=6, params={"format": "json"})
            kwargs = sessions[0].request.call_args.kwargs
            self.assertFalse(sessions[0].trust_env)
            self.assertTrue(kwargs["verify"])
            self.assertTrue(kwargs["stream"])
            self.assertFalse(kwargs["allow_redirects"])
            self.assertEqual(kwargs["timeout"], (4.0, 6))
            self.assertEqual(kwargs["params"], {"format": "json"})
            self.assertEqual(result.json(), {"ok": True})
            self.assertTrue(fixture.closed)
            sessions[0].__exit__.assert_called_once()

    def test_redirects_are_never_followed_or_retried(self):
        for location in ("http://169.254.169.254/private-token", URL):
            fixture = ResponseFixture(status=302, headers={"Location": location})
            with self.subTest(location=location), transport(fixture) as (sessions, _, _, sleep):
                with self.assertRaises(SecureHTTPError) as error:
                    request(URL, allowed_hosts=HOSTS, attempts=3)
                self.assertEqual(len(sessions), 1)
                self.assertEqual(fixture.chunks_read, 0)
                self.assertTrue(fixture.closed)
                sleep.assert_not_called()
                self.assertNotIn(location, str(error.exception))

    def test_large_declared_length_is_rejected_before_reading(self):
        fixture = ResponseFixture(headers={"Content-Length": "7"})
        with transport(fixture), self.assertRaises(SecureHTTPError):
            request(URL, allowed_hosts=HOSTS, max_bytes=6)
        self.assertEqual(fixture.chunks_read, 0)
        self.assertTrue(fixture.closed)

    def test_missing_forged_compressed_lengths_cannot_evade_stream_limit(self):
        for headers in ({}, {"Content-Length": "2"}, {"Content-Encoding": "gzip", "Content-Length": "2"}):
            fixture = ResponseFixture((b"1234", b"5678", b"must-not-be-read"), headers=headers)
            with self.subTest(headers=headers), transport(fixture), self.assertRaises(SecureHTTPError):
                request(URL, allowed_hosts=HOSTS, max_bytes=6)
            self.assertEqual(fixture.chunks_read, 2)
            self.assertTrue(fixture.closed)

    def test_get_retries_are_bounded_and_dns_is_rechecked(self):
        with transport(requests.Timeout("private-url-and-topic"), ResponseFixture()) as (sessions, _, dns, sleep):
            self.assertEqual(request(URL, allowed_hosts=HOSTS).status_code, 200)
            self.assertEqual(len(sessions), 2)
            self.assertEqual(dns.call_count, 2)
            sleep.assert_called_once()
        with transport(*[requests.ConnectionError("private-token") for _ in range(3)]) as (sessions, _, _, sleep):
            with self.assertRaises(SecureHTTPError) as error:
                request(URL, allowed_hosts=HOSTS, attempts=3)
            self.assertEqual(len(sessions), 3)
            self.assertEqual(sleep.call_count, 2)
            self.assertNotIn("private-", str(error.exception))
            self.assertIsNone(error.exception.__context__)

    def test_get_status_retries_stop_and_preserve_final_retry_after(self):
        fixtures = [ResponseFixture(status=503), ResponseFixture(status=429),
                    ResponseFixture(status=429, headers={"retry-after": "17"})]
        with transport(*fixtures) as (sessions, _, _, sleep):
            result = request(URL, allowed_hosts=HOSTS, attempts=3)
            self.assertEqual(result.status_code, 429)
            self.assertEqual(result.headers.get("Retry-After"), "17")
            self.assertEqual(len(sessions), 3)
            self.assertEqual(sleep.call_count, 2)
        self.assertTrue(all(fixture.closed for fixture in fixtures))

    def test_post_is_never_replayed_and_preserves_429(self):
        with transport(ResponseFixture(status=429, headers={"Retry-After": "23"})) as (sessions, _, _, sleep):
            result = request("https://ntfy.sh/private-topic", method="POST", allowed_hosts={"ntfy.sh"},
                             data="message", attempts=3)
            self.assertEqual(result.status_code, 429)
            self.assertEqual(result.headers.get("retry-after"), "23")
            self.assertEqual(len(sessions), 1)
            sleep.assert_not_called()
        with transport(requests.Timeout("private-topic")) as (sessions, _, _, sleep):
            with self.assertRaises(SecureHTTPError):
                request("https://ntfy.sh/private-topic", method="POST", allowed_hosts={"ntfy.sh"}, attempts=3)
            self.assertEqual(len(sessions), 1)
            sleep.assert_not_called()

    def test_native_json_lists_and_nonfinite_json_rejection(self):
        with transport(ResponseFixture((b'[{"rate":1.25}]',))):
            self.assertEqual(get_json(URL, allowed_hosts=HOSTS), [{"rate": 1.25}])
        for body in (b"private-body-not-json", b'{"value":NaN}', b'{"value":Infinity}', b"\xff"):
            with self.subTest(body=body), self.assertRaises(SecureHTTPError) as error:
                SecureHTTPResponse(200, {}, body).json()
            self.assertEqual(str(error.exception), "Invalid JSON response")
            self.assertTrue(error.exception.__suppress_context__)

    def test_http_errors_do_not_expose_url_body_or_header_values(self):
        with transport(ResponseFixture((b"private-response-body",), status=401)):
            with self.assertRaises(SecureHTTPError) as error:
                get_json(URL + "?token=private-token", allowed_hosts=HOSTS)
            self.assertEqual(str(error.exception), "HTTP status 401")
        with transport(requests.exceptions.InvalidHeader("private-token")):
            with self.assertRaises(SecureHTTPError) as error:
                request(URL, allowed_hosts=HOSTS)
            self.assertEqual(str(error.exception), "HTTP request failed")
            self.assertTrue(error.exception.__suppress_context__)

    def test_invalid_limits_fail_before_network(self):
        for limits in ({"timeout": 0}, {"timeout": float("nan")}, {"timeout": float("inf")},
                       {"timeout": True}, {"max_bytes": 0}, {"max_bytes": MAX_RESPONSE_BYTES + 1},
                       {"max_bytes": True}, {"attempts": 0}, {"attempts": 4}, {"attempts": True}):
            with self.subTest(limits=limits), patch("secure_http.requests.Session") as factory:
                with self.assertRaises(SecureHTTPError):
                    request(URL, allowed_hosts=HOSTS, **limits)
                factory.assert_not_called()
        self.assertEqual(SecurityConfig().max_bytes, 10 * 1024 * 1024)

    def test_target_and_implicit_credentials_headers_cannot_be_overridden(self):
        for name in ("Host", "hOSt", "Cookie", "Proxy-Authorization"):
            with self.subTest(header=name), patch("secure_http.requests.Session") as factory:
                with self.assertRaises(SecureHTTPError):
                    request(URL, allowed_hosts=HOSTS, headers={name: "private-token"})
                factory.assert_not_called()

    def test_elapsed_budget_is_checked_during_streaming(self):
        fixture = ResponseFixture((b"one", b"two"))
        with transport(fixture), patch("secure_http.time.monotonic", side_effect=[0, 0.1, 2]):
            with self.assertRaisesRegex(SecureHTTPError, "time budget"):
                request(URL, allowed_hosts=HOSTS, timeout=1, attempts=1)
        self.assertEqual(fixture.chunks_read, 1)
        self.assertTrue(fixture.closed)

    def test_dns_errors_are_generic(self):
        with patch("secure_http.socket.getaddrinfo", side_effect=OSError("private-host")), patch(
            "secure_http.requests.Session"
        ) as factory:
            with self.assertRaises(SecureHTTPError) as error:
                request(URL, allowed_hosts=HOSTS)
            self.assertEqual(str(error.exception), "HTTP DNS resolution failed")
            self.assertTrue(error.exception.__suppress_context__)
            factory.assert_not_called()


if __name__ == "__main__":
    unittest.main()
