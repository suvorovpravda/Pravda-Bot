import unittest

from alert_gate import AlertGate


class AlertGateTests(unittest.TestCase):
    def update(self, gate, now, *, profitable=True, spread=0.5, rearm=0.3):
        return gate.update(
            "BTC/USDT",
            profitable=profitable,
            spread=spread,
            rearm_threshold=rearm,
            now=now,
        )

    def test_requires_continuous_confirmation_before_triggering(self):
        gate = AlertGate(3)
        self.assertFalse(self.update(gate, 10))
        self.assertFalse(self.update(gate, 12.9))
        self.assertTrue(self.update(gate, 13))
        self.assertTrue(gate.is_active("BTC/USDT"))

    def test_unprofitable_interval_resets_confirmation_timer(self):
        gate = AlertGate(3)
        self.assertFalse(self.update(gate, 10))
        self.assertFalse(self.update(gate, 12, profitable=False))
        self.assertFalse(self.update(gate, 14.9))
        self.assertFalse(self.update(gate, 17.8))
        self.assertTrue(self.update(gate, 17.9))

    def test_hysteresis_rearms_only_below_lower_threshold(self):
        gate = AlertGate(0)
        self.assertTrue(self.update(gate, 1))
        self.assertFalse(self.update(gate, 2, profitable=False, spread=0.35, rearm=0.3))
        self.assertTrue(gate.is_active("BTC/USDT"))
        self.assertFalse(self.update(gate, 3, profitable=False, spread=0.3, rearm=0.3))
        self.assertFalse(gate.is_active("BTC/USDT"))
        self.assertTrue(self.update(gate, 4))

    def test_data_quality_reset_does_not_clear_active_alert_latch(self):
        gate = AlertGate(0)
        self.assertTrue(self.update(gate, 1))
        gate.reset_candidate("BTC/USDT")
        self.assertTrue(gate.is_active("BTC/USDT"))

    def test_cancel_activation_allows_a_fresh_signal(self):
        gate = AlertGate(0)
        self.assertTrue(self.update(gate, 1))
        activation_id = gate.activation_id("BTC/USDT")
        self.assertTrue(gate.cancel_activation("BTC/USDT", activation_id))
        self.assertFalse(gate.is_active("BTC/USDT"))
        self.assertTrue(self.update(gate, 2))

    def test_cancel_activation_does_not_cancel_a_newer_activation(self):
        gate = AlertGate(0)
        self.assertTrue(self.update(gate, 1))
        old_activation_id = gate.activation_id("BTC/USDT")
        self.assertFalse(
            self.update(gate, 2, profitable=False, spread=0.3, rearm=0.3)
        )
        self.assertTrue(self.update(gate, 3))
        new_activation_id = gate.activation_id("BTC/USDT")

        self.assertNotEqual(old_activation_id, new_activation_id)
        self.assertFalse(gate.cancel_activation("BTC/USDT", old_activation_id))
        self.assertEqual(gate.activation_id("BTC/USDT"), new_activation_id)

    def test_reset_candidate_discards_pending_confirmation(self):
        gate = AlertGate(3)
        self.assertFalse(self.update(gate, 10))
        gate.reset_candidate("BTC/USDT")
        self.assertFalse(self.update(gate, 12.9))
        self.assertTrue(self.update(gate, 15.9))


if __name__ == "__main__":
    unittest.main()
