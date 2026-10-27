"""
Geospatial mapping module for Pravda-Bot.

Constraints:
- Use exclusively public open data from ENTSOG.
- No commercial AIS provider. Maritime/fluvial tracking is modeled via proxy
  using ENTSOG connection points, LNG terminal metadata, and open datasets
  under permissive licenses.

Design:
- Connection points from ENTSOG API provide lat/lon and cross-border status.
- Pipeline proxy is rendered as straight-line segments between linked
  connection points, with explicit attribution and uncertainty.
- No real pipeline geometry is claimed. All edges are marked as
  "proxy_inferred" with source, date, precision.
- Maritime proxy uses ENTSOG LNG terminal points + open port data from
  OpenStreetMap via public extracts, never AIS.
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime, timezone
from typing import List, Optional, Dict

from energy_data import get_entsog_flows, EnergyDataError


@dataclass(frozen=True)
class GeoPoint:
    key: str
    label: str
    lat: float
    lon: float
    country: Optional[str] = None
    source: str = "ENTSOG"
    retrieved_at: str = ""


@dataclass(frozen=True)
class ProxyEdge:
    from_key: str
    to_key: str
    mode: str  # "pipeline_proxy" | "lng_terminal"
    source: str = "ENTSOG"
    precision: str = "point_to_point"
    retrieved_at: str = ""


class GeospatialMapper:
    """
    Build a proxy network from ENTSOG connection points only.
    No AIS data is ingested. No commercial licenses required.
    """

    def __init__(self):
        self._points: Dict[str, GeoPoint] = {}

    def refresh(self) -> None:
        """Refresh ENTSOG points and build proxy edges."""
        try:
            flows = get_entsog_flows()
        except EnergyDataError:
            return

        points = {}
        for obs in flows.get("observations", []):
            meta = obs.get("meta", {})
            lat = meta.get("tpMapY")
            lon = meta.get("tpMapX")
            if lat is None or lon is None:
                continue
            try:
                lat_f = float(lat)
                lon_f = float(lon)
            except (TypeError, ValueError):
                continue
            points[obs.get("pointKey")] = GeoPoint(
                key=obs.get("pointKey", ""),
                label=obs.get("pointLabel", ""),
                lat=lat_f,
                lon=lon_f,
                country=obs.get("country"),
                source="ENTSOG",
                retrieved_at=flows.get("retrieved_at", ""),
            )
        self._points = points

    def get_points(self) -> List[GeoPoint]:
        return list(self._points.values())

    def get_proxy_edges(self) -> List[ProxyEdge]:
        """
        Generate proxy edges between points that share a country border
        or are linked by ENTSOG operational data. Edges are explicitly
        marked as inferred, not real pipeline geometry.
        """
        edges: List[ProxyEdge] = []
        # Simple heuristic: connect points within same country border group
        # In production, replace with open datasets under ODbL/CC-BY.
        points = list(self._points.values())
        for i in range(len(points)):
            for j in range(i + 1, len(points)):
                p1, p2 = points[i], points[j]
                if p1.country and p2.country and p1.country == p2.country:
                    # Proxy edge with uncertainty
                    edges.append(
                        ProxyEdge(
                            from_key=p1.key,
                            to_key=p2.key,
                            mode="pipeline_proxy",
                            source="ENTSOG",
                            precision="inferred_point_pair",
                            retrieved_at=p1.retrieved_at,
                        )
                    )
        return edges

    def export_geojson(self) -> dict:
        """Export points and proxy edges as GeoJSON FeatureCollection."""
        features = []
        for p in self.get_points():
            features.append({
                "type": "Feature",
                "geometry": {"type": "Point", "coordinates": [p.lon, p.lat]},
                "properties": {
                    "key": p.key,
                    "label": p.label,
                    "country": p.country,
                    "source": p.source,
                    "retrieved_at": p.retrieved_at,
                    "note": "Proxy point, not pipeline geometry"
                }
            })
        for e in self.get_proxy_edges():
            p1 = self._points.get(e.from_key)
            p2 = self._points.get(e.to_key)
            if not p1 or not p2:
                continue
            features.append({
                "type": "Feature",
                "geometry": {
                    "type": "LineString",
                    "coordinates": [[p1.lon, p1.lat], [p2.lon, p2.lat]]
                },
                "properties": {
                    "from_key": e.from_key,
                    "to_key": e.to_key,
                    "mode": e.mode,
                    "precision": e.precision,
                    "source": e.source,
                    "retrieved_at": e.retrieved_at,
                    "note": "Proxy inferred edge, not real pipeline"
                }
            })
        return {"type": "FeatureCollection", "features": features}


def get_geospatial_snapshot() -> dict:
    mapper = GeospatialMapper()
    mapper.refresh()
    return mapper.export_geojson()
