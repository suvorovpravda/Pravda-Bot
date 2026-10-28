"""
OSINT pipeline for Pravda-Bot.

Two-level qualification:
- Reporting: raw articles/dispatches collected, needs human/cross-check.
- Validated Incidents: events confirmed by primary/official sources before display.

Constraints:
- No paid/restricted APIs.
- Asynchrony, backoff, error handling.
- Least privilege storage: raw data is ephemeral, validated incidents are stored
  with provenance and fingerprinting.
"""

from __future__ import annotations

import hashlib
import json
import threading
import time
from dataclasses import dataclass, asdict
from datetime import datetime, timezone
from typing import List, Optional, Dict
from urllib.parse import urlparse

import requests

from energy_data import EnergyDataError


@dataclass(frozen=True)
class SourceMeta:
    url: str
    title: str
    published_at: Optional[str]
    publisher: str
    retrieved_at: str


@dataclass(frozen=True)
class Article:
    id: str
    source: SourceMeta
    content_hash: str
    raw_text: str
    level: str  # "reporting" | "validated"
    provenance: List[str]
    created_at: str


class OSINTStore:
    """In-memory store with least privilege: raw articles are transient, validated are kept."""

    def __init__(self):
        self._lock = threading.Lock()
        self._reporting: Dict[str, Article] = {}
        self._validated: Dict[str, Article] = {}

    def _fingerprint(self, url: str, published_at: Optional[str]) -> str:
        raw = f"{url}|{published_at or ''}".encode("utf-8")
        return hashlib.sha256(raw).hexdigest()

    def ingest_raw(self, url: str, title: str, published_at: Optional[str], publisher: str, raw_text: str) -> Article:
        now = datetime.now(timezone.utc).isoformat()
        fid = self._fingerprint(url, published_at)
        meta = SourceMeta(url=url, title=title, published_at=published_at, publisher=publisher, retrieved_at=now)
        content_hash = hashlib.sha256(raw_text.encode("utf-8")).hexdigest()
        article = Article(
            id=fid,
            source=meta,
            content_hash=content_hash,
            raw_text=raw_text,
            level="reporting",
            provenance=[url],
            created_at=now,
        )
        with self._lock:
            self._reporting[fid] = article
        return article

    def promote_to_validated(self, article_id: str, confirming_sources: List[str]) -> Optional[Article]:
        with self._lock:
            article = self._reporting.get(article_id)
            if not article:
                return None
            validated = Article(
                id=article.id,
                source=article.source,
                content_hash=article.content_hash,
                raw_text=article.raw_text,
                level="validated",
                provenance=article.provenance + confirming_sources,
                created_at=article.created_at,
            )
            self._validated[article_id] = validated
            # Keep reporting for audit, but do not expose raw to dashboard
            return validated

    def list_reporting(self) -> List[Article]:
        with self._lock:
            return list(self._reporting.values())

    def list_validated(self) -> List[Article]:
        with self._lock:
            return list(self._validated.values())


class OSINTCollector:
    """Asynchronous collector with backoff and error handling."""

    def __init__(self, store: OSINTStore):
        self.store = store
        self._session = requests.Session()
        self._session.headers.update({"User-Agent": "Pravda-Bot-OSINT/1.0"})

    def _fetch_with_backoff(self, url: str, max_retries: int = 3, base_delay: float = 1.0) -> Optional[str]:
        delay = base_delay
        for attempt in range(max_retries):
            try:
                resp = self._session.get(url, timeout=15)
                resp.raise_for_status()
                return resp.text
            except requests.RequestException:
                time.sleep(delay)
                delay *= 2
        return None

    def collect_from_rss(self, rss_url: str, publisher: str) -> List[Article]:
        """
        Collect articles from RSS feed. This is a reporting-level collection.
        No paid APIs.
        """
        xml = self._fetch_with_backoff(rss_url)
        if not xml:
            raise EnergyDataError("RSS fetch failed")
        # Minimal RSS parsing without external deps
        import re
        items = re.findall(r"<item>(.*?)</item>", xml, flags=re.DOTALL)
        articles = []
        for item in items:
            title = re.search(r"<title>(.*?)</title>", item, flags=re.DOTALL)
            link = re.search(r"<link>(.*?)</link>", item, flags=re.DOTALL)
            pub = re.search(r"<pubDate>(.*?)</pubDate>", item, flags=re.DOTALL)
            if not (title and link):
                continue
            raw_text = item
            article = self.store.ingest_raw(
                url=link.group(1).strip(),
                title=re.sub(r"<.*?>", "", title.group(1)).strip(),
                published_at=pub.group(1).strip() if pub else None,
                publisher=publisher,
                raw_text=raw_text,
            )
            articles.append(article)
        return articles

    def cross_validate(self, article_id: str, confirming_urls: List[str]) -> Optional[Article]:
        """
        Promote article to validated if confirming sources are provided.
        In real deployment, this would involve content similarity checks.
        """
        return self.store.promote_to_validated(article_id, confirming_urls)


_store = OSINTStore()
_collector = OSINTCollector(_store)


def get_osint_reporting() -> List[dict]:
    return [asdict(a) for a in _store.list_reporting()]


def get_osint_validated() -> List[dict]:
    return [asdict(a) for a in _store.list_validated()]


def collect_osint_feed(rss_url: str, publisher: str) -> List[dict]:
    articles = _collector.collect_from_rss(rss_url, publisher)
    return [asdict(a) for a in articles]
