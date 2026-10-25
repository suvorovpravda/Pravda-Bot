import unittest

from bot_config import load_config


class BotConfigTests(unittest.TestCase):
    def test_defaults_preserve_expected_behavior(self):
        config = load_config({})
        self.assertEqual(config.min_spread_threshold, 0.05)
        self.assertEqual(config.max_allowed_spread_percent, 5.0)
        self.assertEqual(config.spread_log_interval_seconds, 10)
        self.assertEqual(config.dashboard_interval_seconds, 30)
        self.assertEqual(config.price_alert_cooldown_seconds, 5)
        self.assertEqual(config.ntfy_global_cooldown_seconds, 10)
        self.assertEqual(config.ntfy_topic_url, "")
        self.assertEqual(config.paper_mode, "portfolio")
        self.assertEqual(config.paper_starting_balance_usdt, 1000)
        self.assertEqual(config.paper_position_percent, 10)
        self.assertEqual(config.alert_confirmation_seconds, 3)
        self.assertEqual(config.market_data_stale_seconds, 15)

    def test_custom_settings_are_loaded(self):
        config = load_config(
            {
                "LM_STUDIO_API_BASE": "http://localhost:1234/v1/",
                "LM_MODEL_NAME": "local-model",
                "MIN_SPREAD_THRESHOLD": "0.25",
                "SPREAD_LOG_INTERVAL_SECONDS": "3",
                "DASHBOARD_INTERVAL_SECONDS": "15",
                "PRICE_ALERT_COOLDOWN_SECONDS": "0",
                "NTFY_GLOBAL_COOLDOWN_SECONDS": "2",
                "NTFY_TOPIC_URL": "https://ntfy.sh/random",
            }
        )
        self.assertEqual(config.lm_url, "http://localhost:1234/v1")
        self.assertEqual(config.min_spread_threshold, 0.25)
        self.assertEqual(config.spread_log_interval_seconds, 3)
        self.assertEqual(config.dashboard_interval_seconds, 15)
        self.assertEqual(config.price_alert_cooldown_seconds, 0)

    def test_invalid_numeric_settings_are_rejected(self):
        for key, value in (
            ("MIN_SPREAD_THRESHOLD", "-0.1"),
            ("MIN_SPREAD_THRESHOLD", "nan"),
            ("MAX_ALLOWED_SPREAD_PERCENT", "0"),
            ("MAX_ALLOWED_SPREAD_PERCENT", "nan"),
            ("SPREAD_LOG_INTERVAL_SECONDS", "0"),
            ("DASHBOARD_INTERVAL_SECONDS", "inf"),
            ("PRICE_ALERT_COOLDOWN_SECONDS", "-1"),
            ("PAPER_POSITION_PERCENT", "101"),
            ("PAPER_STARTING_BALANCE_USDT", "0"),
            ("MARKET_DATA_MAX_SKEW_SECONDS", "nan"),
        ):
            with self.subTest(key=key, value=value):
                with self.assertRaisesRegex(ValueError, key):
                    load_config({key: value})

    def test_invalid_urls_and_empty_model_are_rejected(self):
        for environment, expected in (
            ({"LM_STUDIO_API_BASE": "file:///tmp/model"}, "LM_STUDIO_API_BASE"),
            ({"LM_STUDIO_API_BASE": "http://user:pass@localhost"}, "LM_STUDIO_API_BASE"),
            ({"LM_STUDIO_API_BASE": "http://localhost:bad"}, "LM_STUDIO_API_BASE"),
            ({"NTFY_TOPIC_URL": "http://ntfy.sh/topic"}, "NTFY_TOPIC_URL"),
            ({"LM_MODEL_NAME": " "}, "LM_MODEL_NAME"),
            ({"PAPER_MODE": "live"}, "PAPER_MODE"),
        ):
            with self.subTest(environment=environment):
                with self.assertRaisesRegex(ValueError, expected):
                    load_config(environment)


if __name__ == "__main__":
    unittest.main()
