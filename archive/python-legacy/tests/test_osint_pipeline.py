import unittest
from osint_pipeline import OSINTStore, OSINTCollector


class OSINTStoreTests(unittest.TestCase):
    def setUp(self):
        self.store = OSINTStore()

    def test_ingest_raw_creates_reporting_article(self):
        article = self.store.ingest_raw(
            url="https://example.com/a",
            title="Test",
            published_at="2026-10-27",
            publisher="TestPub",
            raw_text="content"
        )
        self.assertEqual(article.level, "reporting")
        self.assertEqual(len(self.store.list_reporting()), 1)

    def test_promote_to_validated_moves_article(self):
        article = self.store.ingest_raw(
            url="https://example.com/b",
            title="Test",
            published_at=None,
            publisher="Pub",
            raw_text="c"
        )
        validated = self.store.promote_to_validated(article.id, ["https://official.gov/confirm"])
        self.assertIsNotNone(validated)
        self.assertEqual(validated.level, "validated")
        self.assertEqual(len(self.store.list_validated()), 1)

    def test_least_privilege_raw_stays_separate(self):
        self.store.ingest_raw("https://x", "t", None, "p", "r")
        self.assertEqual(len(self.store.list_reporting()), 1)
        self.assertEqual(len(self.store.list_validated()), 0)


class OSINTCollectorTests(unittest.TestCase):
    def setUp(self):
        self.store = OSINTStore()
        self.collector = OSINTCollector(self.store)

    def test_collect_from_rss_parses_items(self):
        rss = b"""<?xml version="1.0"?><rss><channel><item><title>A</title><link>http://a</link></item></channel></rss>"""
        with unittest.mock.patch.object(self.collector._session, "get") as mock_get:
            mock_resp = unittest.mock.Mock()
            mock_resp.raise_for_status = unittest.mock.Mock()
            mock_resp.text = rss.decode()
            mock_get.return_value = mock_resp
            articles = self.collector.collect_from_rss("http://feed", "Pub")
            self.assertEqual(len(articles), 1)
            self.assertEqual(articles[0].source.publisher, "Pub")


if __name__ == "__main__":
    unittest.main()
