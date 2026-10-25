import unittest
from unittest.mock import patch

from security_and_fees import (
    calculate_net_spread,
    check_security_guardrails,
)


class SecurityAndFeesTests(unittest.TestCase):
    def test_net_spread_uses_existing_fee_estimate_in_percentage_points(self):
        self.assertAlmostEqual(calculate_net_spread(0.5), 0.14)
        self.assertAlmostEqual(calculate_net_spread(0.1), -0.26)

    def test_net_spread_rejects_invalid_values(self):
        for gross, fees in ((float("nan"), 0.36), (1, float("inf")), (-1, 0.36), (1, -0.1)):
            with self.subTest(gross=gross, fees=fees):
                with self.assertRaises(ValueError):
                    calculate_net_spread(gross, fees)

    def test_guardrail_accepts_positive_spreads_at_or_below_limit(self):
        self.assertTrue(check_security_guardrails(0.01))
        self.assertTrue(check_security_guardrails(5.0))

    def test_guardrail_rejects_nonpositive_and_nonfinite_spreads(self):
        for spread in (0, -0.1, float("nan"), float("inf")):
            with self.subTest(spread=spread):
                self.assertFalse(check_security_guardrails(spread))

    def test_guardrail_rejects_large_spreads_and_logs_warning(self):
        with patch("security_and_fees.logger.warning") as warning:
            self.assertFalse(check_security_guardrails(5.01))
        warning.assert_called_once()

    def test_guardrail_rejects_invalid_limits(self):
        for limit in (0, -1, float("nan"), float("inf")):
            with self.subTest(limit=limit):
                with self.assertRaises(ValueError):
                    check_security_guardrails(1, limit)


if __name__ == "__main__":
    unittest.main()
