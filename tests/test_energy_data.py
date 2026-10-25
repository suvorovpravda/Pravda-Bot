import unittest
from datetime import datetime, timezone
from unittest.mock import Mock, patch

import energy_data


class EnergyDataTests(unittest.TestCase):
    def test_entsog_filters_to_reported_cross_border_values(self):
        now = datetime(2026, 10, 25, 12, tzinfo=timezone.utc)
        updated = now.isoformat()
        points = {
            "connectionpoints": [
                {
                    "pointKey": "BORDER-1",
                    "pointLabel": "Border One",
                    "isCrossBorder": True,
                    "tpMapX": 12,
                    "tpMapY": 34,
                    "importFromCountryLabel": "France",
                },
                {
                    "pointKey": "LOCAL-1",
                    "pointLabel": "Local",
                    "isCrossBorder": False,
                },
            ]
        }
        flows = {
            "operationalData": [
                {
                    "pointKey": "BORDER-1",
                    "pointLabel": "Border One",
                    "directionKey": "entry",
                    "value": "1250000",
                    "unit": "kWh/d",
                    "periodFrom": "2026-10-25T06:00:00+02:00",
                    "lastUpdateDateTime": updated,
                    "isNA": 0,
                },
                {
                    "pointKey": "BORDER-1",
                    "directionKey": "exit",
                    "value": "",
                    "unit": "kWh/d",
                    "periodFrom": "2026-10-25T06:00:00+02:00",
                    "lastUpdateDateTime": updated,
                    "isNA": 1,
                },
                {
                    "pointKey": "LOCAL-1",
                    "directionKey": "entry",
                    "value": "999",
                    "unit": "kWh/d",
                    "periodFrom": "2026-10-25T06:00:00+02:00",
                    "lastUpdateDateTime": updated,
                    "isNA": 0,
                },
            ]
        }
        responses = [points, flows]
        with patch.dict(energy_data._cache, {}, clear=True), patch(
            "energy_data.requests.get"
        ) as get:
            get.side_effect = [
                self.response(payload) for payload in responses
            ]
            result = energy_data.get_entsog_flows(now=now)

        self.assertEqual(result["status"], "live")
        self.assertEqual(len(result["observations"]), 1)
        self.assertEqual(result["observations"][0]["value"], 1250000.0)
        self.assertEqual(result["observations"][0]["country"], "France")

    def test_stale_source_update_is_not_labelled_live(self):
        now = datetime(2026, 10, 25, 12, tzinfo=timezone.utc)
        points = {
            "connectionpoints": [
                {"pointKey": "BORDER-1", "isCrossBorder": True}
            ]
        }
        flows = {
            "operationalData": [
                {
                    "pointKey": "BORDER-1",
                    "value": "10",
                    "periodFrom": "2026-10-20T00:00:00Z",
                    "lastUpdateDateTime": "2026-10-20T00:00:00Z",
                    "isNA": 0,
                }
            ]
        }
        with patch.dict(energy_data._cache, {}, clear=True), patch(
            "energy_data.requests.get"
        ) as get:
            get.side_effect = [self.response(points), self.response(flows)]
            result = energy_data.get_entsog_flows(now=now)

        self.assertEqual(result["status"], "stale")

    def test_news_feed_uses_bbc_headlines_when_gdelt_is_unavailable(self):
        rss = """<?xml version="1.0"?>
        <rss version="2.0"><channel>
          <item>
            <title>Pipeline supply disruption reported</title>
            <description>Officials are reviewing the impact.</description>
            <link>https://example.org/story</link>
            <pubDate>Sun, 25 Oct 2026 07:00:00 GMT</pubDate>
          </item>
          <item>
            <title>Local sports results</title>
            <description>Latest sports report.</description>
            <link>https://example.org/sports</link>
          </item>
        </channel></rss>"""
        response = Mock()
        response.text = rss
        response.raise_for_status.return_value = None
        with patch.dict(energy_data._cache, {}, clear=True), patch(
            "energy_data._cached_json", side_effect=energy_data.EnergyDataError("offline")
        ), patch("energy_data.requests.get", return_value=response):
            result = energy_data.get_geopolitical_news()

        self.assertEqual(result["source"], "BBC News · World")
        self.assertEqual(len(result["articles"]), 1)
        self.assertEqual(result["articles"][0]["title"], "Pipeline supply disruption reported")

    @staticmethod
    def response(payload):
        response = Mock()
        response.json.return_value = payload
        response.raise_for_status.return_value = None
        return response


if __name__ == "__main__":
    unittest.main()
