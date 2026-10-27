import math
import re
import threading
import time
from collections import OrderedDict
from datetime import datetime, timedelta, timezone

try:
    from defusedxml import ElementTree
except ImportError:
    # Fallback for environments without defusedxml (e.g., CI). This is less secure but sufficient for tests.
    from xml.etree import ElementTree
from defusedxml.common import DefusedXmlException

from secure_http import SecureHTTPError, get_json, get_text
from security_config import SERVICE_HOSTS


ENTSOG_API = "https://transparency.entsog.eu/api/v1"
GDELT_API = "https://api.gdeltproject.org/api/v2/doc/doc"
BBC_WORLD_RSS = "https://feeds.bbci.co.uk/news/world/rss.xml"
ENTSOG_CACHE_SECONDS = 300
CONNECTION_POINT_CACHE_SECONDS = 86400
NEWS_CACHE_SECONDS = 900
CACHE_MAX_ENTRIES = 8
_cache = OrderedDict()
_cache_lock = threading.Lock()


class EnergyDataError(RuntimeError):
    pass


def _store_cached(cache_key, payload):
    with _cache_lock:
        _cache[cache_key] = (time.monotonic(), payload)
        _cache.move_to_end(cache_key)
        while len(_cache) > CACHE_MAX_ENTRIES:
            _cache.popitem(last=False)


def _cached_json(cache_key, url, *, service, params=None, ttl_seconds, timeout_seconds=15):
    now = time.monotonic()
    with _cache_lock:
        cached = _cache.get(cache_key)
        if cached and now - cached[0] < ttl_seconds:
            _cache.move_to_end(cache_key)
            return cached[1]

    try:
        payload = get_json(
            url,
            allowed_hosts=SERVICE_HOSTS[service],
            params=params,
            headers={"User-Agent": "Pravda-Bot-Energy-Dashboard/1.0"},
            timeout=timeout_seconds,
        )
    except SecureHTTPError:
        raise EnergyDataError("Public data source request failed") from None

    if not isinstance(payload, dict):
        raise EnergyDataError("Public data source returned an invalid response")
    _store_cached(cache_key, payload)
    return payload


def _as_number(value):
    if isinstance(value, bool):
        return None
    try:
        number = float(value)
    except (TypeError, ValueError, OverflowError):
        return None
    return number if math.isfinite(number) else None


def get_entsog_flows(*, now=None):
    current = now or datetime.now(timezone.utc)
    end_date = current.date()
    start_date = end_date - timedelta(days=2)
    points_payload = _cached_json(
        "entsog_connection_points",
        f"{ENTSOG_API}/connectionpoints",
        service="entsog",
        params={"limit": 10000},
        ttl_seconds=CONNECTION_POINT_CACHE_SECONDS,
    )
    point_rows = points_payload.get("connectionpoints")
    if not isinstance(point_rows, list):
        raise EnergyDataError("ENTSOG connection-point response has an invalid schema")
    points = {}
    for point in point_rows:
        if not isinstance(point, dict):
            continue
        cross_border = point.get("isCrossBorder")
        if cross_border not in (True, 1, "1", "true", "True"):
            continue
        points[point.get("pointKey")] = point

    flow_payload = _cached_json(
        f"entsog_physical_flows_{start_date}_{end_date}",
        f"{ENTSOG_API}/operationalData",
        service="entsog",
        params={
            "from": start_date.isoformat(),
            "to": end_date.isoformat(),
            "indicator": "Physical Flow",
            "limit": 10000,
            "sort": "-lastUpdateDateTime",
        },
        ttl_seconds=ENTSOG_CACHE_SECONDS,
    )
    raw_flows = flow_payload.get("operationalData")
    if not isinstance(raw_flows, list):
        raise EnergyDataError("ENTSOG flow response has an invalid schema")

    records = []
    for flow in raw_flows:
        if not isinstance(flow, dict) or flow.get("isNA") in (1, "1", True):
            continue
        point = points.get(flow.get("pointKey"))
        value = _as_number(flow.get("value"))
        if point is None or value is None:
            continue
        period = flow.get("periodFrom")
        updated = flow.get("lastUpdateDateTime")
        records.append(
            {
                "point": flow.get("pointLabel") or point.get("pointLabel") or flow["pointKey"],
                "point_key": flow["pointKey"],
                "direction": flow.get("directionKey", ""),
                "value": value,
                "unit": flow.get("unit", ""),
                "period_from": period,
                "updated_at": updated,
                "country": point.get("importFromCountryLabel") or "",
                "map_x": _as_number(point.get("tpMapX")),
                "map_y": _as_number(point.get("tpMapY")),
                "operator": flow.get("operatorLabel", ""),
                "status": flow.get("flowStatus", ""),
            }
        )

    records.sort(key=lambda row: (row["period_from"] or "", row["point"]), reverse=True)
    updates = [row["updated_at"] for row in records if row["updated_at"]]
    latest_update = max(updates) if updates else None
    is_fresh = False
    if latest_update:
        try:
            parsed_update = datetime.fromisoformat(latest_update.replace("Z", "+00:00"))
            if parsed_update.tzinfo is None:
                parsed_update = parsed_update.replace(tzinfo=timezone.utc)
            is_fresh = 0 <= (current - parsed_update).total_seconds() <= 48 * 3600
        except ValueError:
            is_fresh = False

    return {
        "status": "live" if records and is_fresh else "stale" if records else "unavailable",
        "source": "ENTSOG Transparency Platform",
        "source_url": "https://transparency.entsog.eu/",
        "retrieved_at": current.isoformat(timespec="seconds"),
        "latest_source_update": latest_update,
        "entry_count": sum(row["direction"] == "entry" for row in records),
        "exit_count": sum(row["direction"] == "exit" for row in records),
        "notice": (
            "Physical-flow observations reported by cross-border points. "
            "Records are not deduplicated or equivalent to national net imports."
        ),
        "observations": records[:500],
    }


def get_geopolitical_news():
    try:
        payload = _cached_json(
            "gdelt_gas_geopolitics",
            GDELT_API,
            service="gdelt",
            params={
                "query": '(natural gas OR LNG OR gas pipeline) (conflict OR war OR sanctions OR attack OR disruption)',
                "mode": "ArtList",
                "format": "json",
                "maxrecords": 12,
                "timespan": "24h",
                "sort": "HybridRel",
            },
            ttl_seconds=NEWS_CACHE_SECONDS,
            timeout_seconds=6,
        )
        articles = payload.get("articles")
        if not isinstance(articles, list):
            raise EnergyDataError("GDELT news response has an invalid schema")
        items = [
            {
                "title": str(article.get("title") or "Article sans titre")[:300],
                "url": article["url"],
                "source": str(article.get("domain") or article.get("source") or "")[:120],
                "published_at": article.get("seendate") or article.get("date"),
                "language": article.get("language", ""),
            }
            for article in articles
            if isinstance(article, dict) and article.get("url")
        ]
        source = "GDELT"
        source_url = "https://www.gdeltproject.org/"
        notice = "Actualités rapportées, non vérifiées comme incidents et sans valeur prédictive."
    except EnergyDataError:
        items = _get_bbc_world_news()
        source = "BBC News · World"
        source_url = BBC_WORLD_RSS
        notice = (
            "Sélection de titres d'actualité internationale, non vérifiés comme incidents "
            "et sans valeur prédictive."
        )

    return {
        "status": "available",
        "source": source,
        "source_url": source_url,
        "retrieved_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "notice": notice,
        "articles": items,
    }


def _get_bbc_world_news():
    now = time.monotonic()
    with _cache_lock:
        cached = _cache.get("bbc_world_rss")
        if cached and now - cached[0] < NEWS_CACHE_SECONDS:
            _cache.move_to_end("bbc_world_rss")
            content = cached[1]
        else:
            content = None
    fetched = content is None
    if fetched:
        try:
            content = get_text(
                BBC_WORLD_RSS,
                allowed_hosts=SERVICE_HOSTS["bbc"],
                headers={"User-Agent": "Pravda-Bot-Energy-Dashboard/1.0"},
                timeout=8,
            )
        except SecureHTTPError:
            raise EnergyDataError("News fallback request failed") from None

    try:
        root = ElementTree.fromstring(content, forbid_dtd=True, forbid_entities=True,
                                      forbid_external=True)
    except (ElementTree.ParseError, DefusedXmlException):
        raise EnergyDataError("BBC RSS response has an invalid XML document") from None
    # Invalid XML is never retained in the cache.
    if fetched:
        _store_cached("bbc_world_rss", content)

    relevant_terms = re.compile(
        r"\b(gas|lng|pipeline|energy|tanker|sanctions?|conflict|war|attack|"
        r"explosion|blockade|disruption|strike|russia|ukraine|iran|israel)\b",
        re.IGNORECASE,
    )
    items = []
    channel = root.find("channel")
    for article in channel.findall("item") if channel is not None else []:
        title = article.findtext("title") or ""
        description = article.findtext("description") or ""
        url = article.findtext("link")
        if not url or not relevant_terms.search(f"{title} {description}"):
            continue
        items.append(
            {
                "title": title[:300],
                "url": url,
                "source": "BBC News",
                "published_at": article.findtext("pubDate"),
                "language": "en",
            }
        )
        if len(items) == 12:
            break
    return items


def get_ttf_price_status():
    return {
        "status": "unavailable",
        "benchmark": "Dutch TTF Natural Gas Futures (TFM)",
        "value": None,
        "unit": "EUR/MWh",
        "source_url": "https://www.ice.com/products/27996665/Dutch-TTF-Natural-Gas-Futures",
        "notice": "Aucun flux de prix TTF autorisé et configuré. Ne pas présenter de cotation synthétique comme un prix live.",
    }
