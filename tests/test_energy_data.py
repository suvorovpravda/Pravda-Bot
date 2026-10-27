import unittest
from datetime import datetime, timedelta, timezone
from unittest.mock import patch

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
            "energy_data.get_json"
        ) as get:
            get.side_effect = responses
            result = energy_data.get_entsog_flows(now=now)

        self.assertEqual(result["status"], "live")
        self.assertEqual(len(result["observations"]), 1)
        self.assertEqual(result["observations"][0]["value"], 1250000.0)
        self.assertEqual(result["observations"][0]["country"], "France")
        self.assertTrue(all(call.kwargs["allowed_hosts"] == frozenset({"transparency.entsog.eu"})
                            for call in get.call_args_list))

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
            "energy_data.get_json"
        ) as get:
            get.side_effect = [points, flows]
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
        with patch.dict(energy_data._cache, {}, clear=True), patch(
            "energy_data._cached_json", side_effect=energy_data.EnergyDataError("offline")
        ), patch("energy_data.get_text", return_value=rss) as get:
            result = energy_data.get_geopolitical_news()

        self.assertEqual(result["source"], "BBC News · World")
        self.assertEqual(len(result["articles"]), 1)
        self.assertEqual(result["articles"][0]["title"], "Pipeline supply disruption reported")
        self.assertEqual(get.call_args.kwargs["allowed_hosts"], frozenset({"feeds.bbci.co.uk"}))

    def test_entsog_future_source_dates_are_not_live(self):
        now = datetime(2026, 10, 26, tzinfo=timezone.utc)
        points = {"connectionpoints": [{"pointKey": "A", "isCrossBorder": True}]}
        for offset, expected in ((1, "stale"), (0, "live"), (-48 * 3600, "live"),
                                 (-48 * 3600 - 1, "stale")):
            updated = (now + timedelta(seconds=offset)).isoformat()
            flows = {"operationalData": [{"pointKey": "A", "value": "12",
                                         "lastUpdateDateTime": updated}]}
            with self.subTest(offset=offset), patch(
                "energy_data._cached_json", side_effect=[points, flows]
            ):
                self.assertEqual(energy_data.get_entsog_flows(now=now)["status"], expected)

    def test_cache_is_bounded_across_ninety_daily_windows(self):
        now = datetime(2026, 10, 26, tzinfo=timezone.utc)
        with patch.dict(energy_data._cache, {}, clear=True), patch(
            "energy_data.time.monotonic", return_value=100
        ), patch("energy_data.get_json") as get:
            get.side_effect = lambda url, **kwargs: (
                {"connectionpoints": []} if url.endswith("/connectionpoints") else {"operationalData": []}
            )
            for day in range(90):
                energy_data.get_entsog_flows(now=now + timedelta(days=day))
            self.assertLessEqual(len(energy_data._cache), 8)
            self.assertEqual(get.call_count, 91)
            energy_data.get_entsog_flows(now=now + timedelta(days=89))
            self.assertEqual(get.call_count, 91)
            energy_data.get_entsog_flows(now=now)
            self.assertEqual(get.call_count, 92)

    def test_rss_dtd_and_entities_fail_closed_without_caching(self):
        documents = (
            '<!DOCTYPE rss><rss><channel/></rss>',
            '<!DOCTYPE rss [<!ENTITY x "gas">]><rss><channel><item><title>&x;</title></item></channel></rss>',
            '<!DOCTYPE rss [<!ENTITY x SYSTEM "file:///private-file">]><rss><channel>&x;</channel></rss>',
            '<!DOCTYPE rss SYSTEM "https://private.invalid/a.dtd"><rss><channel/></rss>',
            '<rss><channel>',
        )
        for content in documents:
            with self.subTest(content=content), patch.dict(energy_data._cache, {}, clear=True), patch(
                "energy_data.get_text", return_value=content
            ):
                with self.assertRaises(energy_data.EnergyDataError) as error:
                    energy_data._get_bbc_world_news()
                self.assertEqual(str(error.exception), "BBC RSS response has an invalid XML document")
                self.assertNotIn("bbc_world_rss", energy_data._cache)

    def test_rss_cache_hits_do_not_extend_the_fetch_ttl(self):
        content = '<rss><channel><item><title>Gas &amp; energy</title><link>https://example.org</link></item></channel></rss>'
        with patch.dict(energy_data._cache, {}, clear=True), patch(
            "energy_data.time.monotonic", side_effect=[0, 0, 100, 901, 901]
        ), patch("energy_data.get_text", return_value=content) as get:
            self.assertEqual(len(energy_data._get_bbc_world_news()), 1)
            self.assertEqual(len(energy_data._get_bbc_world_news()), 1)
            self.assertEqual(len(energy_data._get_bbc_world_news()), 1)
            self.assertEqual(get.call_count, 2)

    def test_http_errors_expose_no_upstream_details(self):
        from secure_http import SecureHTTPError
        with patch.dict(energy_data._cache, {}, clear=True), patch(
            "energy_data.get_json", side_effect=SecureHTTPError("private-upstream-detail")
        ):
            with self.assertRaises(energy_data.EnergyDataError) as error:
                energy_data.get_entsog_flows()
            self.assertEqual(str(error.exception), "Public data source request failed")
            self.assertTrue(error.exception.__suppress_context__)


if __name__ == "__main__":
    unittest.main()
