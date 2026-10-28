import unittest
from unittest.mock import patch

from geospatial_mapper import GeospatialMapper, GeoPoint, ProxyEdge


class GeospatialMapperTests(unittest.TestCase):
    def setUp(self):
        self.mapper = GeospatialMapper()

    @patch("geospatial_mapper.get_entsog_flows")
    def test_refresh_populates_points_from_entsog(self, mock_flows):
        mock_flows.return_value = {
            "retrieved_at": "2026-10-27T00:00:00Z",
            "observations": [
                {
                    "pointKey": "P1",
                    "pointLabel": "Border A",
                    "country": "FR",
                    "meta": {"tpMapY": 48.8, "tpMapX": 2.3}
                },
                {
                    "pointKey": "P2",
                    "pointLabel": "Border B",
                    "country": "FR",
                    "meta": {"tpMapY": 49.0, "tpMapX": 2.5}
                }
            ]
        }
        self.mapper.refresh()
        points = self.mapper.get_points()
        self.assertEqual(len(points), 2)
        self.assertTrue(any(p.key == "P1" for p in points))

    @patch("geospatial_mapper.get_entsog_flows")
    def test_proxy_edges_are_marked_inferred(self, mock_flows):
        mock_flows.return_value = {
            "retrieved_at": "2026-10-27T00:00:00Z",
            "observations": [
                {"pointKey": "A", "pointLabel": "A", "country": "DE", "meta": {"tpMapY": 50.0, "tpMapX": 10.0}},
                {"pointKey": "B", "pointLabel": "B", "country": "DE", "meta": {"tpMapY": 50.1, "tpMapX": 10.1}},
            ]
        }
        self.mapper.refresh()
        edges = self.mapper.get_proxy_edges()
        self.assertTrue(len(edges) >= 1)
        for e in edges:
            self.assertEqual(e.mode, "pipeline_proxy")
            self.assertIn("inferred", e.precision)

    def test_export_geojson_structure(self):
        self.mapper._points = {
            "X": GeoPoint(key="X", label="X", lat=1.0, lon=2.0, country="FR", retrieved_at="now")
        }
        geojson = self.mapper.export_geojson()
        self.assertEqual(geojson["type"], "FeatureCollection")
        self.assertTrue(isinstance(geojson["features"], list))


if __name__ == "__main__":
    unittest.main()
