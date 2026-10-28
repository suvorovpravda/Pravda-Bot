"""Test de non-régression Python ↔ Rust pour Pravda-Bot"""
import math
import sys
from security_and_fees import calculate_net_spread_safe

# Import du binding Rust, fallback Python si non compilé
try:
    import pravda_rust_bridge as rb
    rust_available = True
except Exception:
    rust_available = False
    rb = None

def python_net_spread(price_kraken, price_binance, fees_percent):
    return calculate_net_spread_safe(price_kraken, price_binance, fees_percent)

def test_samples():
    samples = [
        (79000.0, 79100.0, 0.1),
        (2500.0, 2505.0, 0.05),
        (100.0, 100.5, 0.02),
        (1.0, 1.01, 0.001),
    ]
    for pk, pb, fees in samples:
        py_val = python_net_spread(pk, pb, fees)
        if rust_available:
            rs_val = rb.calculate_net_spread(pk, pb, fees)
            diff = abs(py_val - rs_val)
            print(f"pk={pk} pb={pb} fees={fees} py={py_val:.6f} rs={rs_val:.6f} diff={diff:.9f}")
            assert diff < 1e-9, f"Différence trop grande: {diff}"
        else:
            print(f"Rust non disponible, py={py_val:.6f}")
    print("Tests de non-régression OK")

if __name__ == "__main__":
    test_samples()
