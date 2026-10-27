"""
Maritime proxy module for Pravda-Bot.

No commercial AIS provider is used. All data comes from open sources:
- OpenStreetMap via Overpass API for ports, LNG terminals, and related infrastructure
- ENTSOG connection points for onshore links

This is a proxy, not real-time AIS tracking.
"""

from __future__ import annotations

import time
from dataclasses import dataclass
from typing import List, Dict
import requests

OVERPASS_URL = "https://overpass-api.de/api/interpreter"


@dataclass(frozen=True)
class MaritimeFacility:
    id: str
    name: str
    type: str  # port, lng_terminal, etc.
    lat: float
    lon: float
    source: str = "OSM"
    retrieved_at: str = ""


class MaritimeProxy:
    def __init__(self):
        self._facilities: List[MaritimeFacility] = []
        self._last_fetch: float = 0.0
        self._cache_ttl = 86400  # 24h

    def _fetch_overpass(self, query: str) -> dict:
        try:
            resp = requests.post(OVERPASS_URL, data={"data": query}, timeout=30)
            resp.raise_for_status()
            return resp.json()
        except requests.RequestException:
            return {}

    def refresh(self) -> None:
        now = time.monotonic()
        if now - self._last_fetch < self._cache_ttl and self._facilities:
            return
        # Query for ports and LNG terminals within Europe
        query = """
        [out:json][timeout:60];
        (
          node["port"="harbour"](around:5000000,50,10);
          node["seamark:type"="terminal"](around:5000000,50,10);
          way["man_made"="lng_terminal"](around:5000000,50,10);
        );
        out center tags;
        """
        data = self._fetch_overpass(query)
        facilities = []
        for el in data.get("elements", []):
            tags = el.get("tags", {})
            name = tags.get("name", "Unnamed")
            typ = "port"
            if "lng_terminal" in tags.get("man_made", ""):
                typ = "lng_terminal"
            elif tags.get("seamark:type") == "terminal":
                typ = "terminal"
            lat = el.get("lat") or el.get("center", {}).get("lat")
            lon = el.get("lon") or el.get("center", {}).get("lon")
            if lat is None or lon is None:
                continue
            facilities.append(MaritimeFacility(
                id=str(el.get("id")),
                name=name,
                type=typ,
                lat=float(lat),
                lon=float(lon),
                source="OSM Overpass",
                retrieved_at=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
            ))
        self._facilities = facilities
        self._last_fetch = now

    def get_facilities(self) -> List[MaritimeFacility]:
        self.refresh()
        return self._facilities

    def export_geojson(self) -> dict:
        features = []
        for f in self.get_facilities():
            features.append({
                "type": "Feature",
                "geometry": {"type": "Point", "coordinates": [f.lon, f.lat]},
                "properties": {
                    "id": f.id,
                    "name": f.name,
                    "type": f.type,
                    "source": f.source,
                    "note": "Proxy open data, not real-time AIS"
                }
            })
        return {"type": "FeatureCollection", "features": features}


def get_maritime_proxy_snapshot() -> dict:
    proxy = MaritimeProxy()
    return proxy.export_geojson()
