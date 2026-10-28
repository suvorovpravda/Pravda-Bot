use pyo3::prelude::*;

/// Calculates the net spread between Kraken and Binance prices.
/// Matches Python security_and_fees.calculate_net_spread_safe formula exactly:
/// - Uses minimum price as denominator (not mean)
/// - Returns ratio, NOT percentage (*100 removed)
/// Formula: spread = (price_kraken - price_binance) / min_price - fee_rate
#[pyfunction]
fn calculate_net_spread(price_kraken: f64, price_binance: f64, fees_percent: f64) -> PyResult<f64> {
    // Validation: prices must be finite and positive (P2-2)
    if !price_kraken.is_finite() || !price_binance.is_finite() || price_kraken <= 0.0 || price_binance <= 0.0 {
        return Err(pyo3::exceptions::PyValueError::new_err("prices must be finite and positive"));
    }
    
    // Validation: fees_percent must be finite and non-negative (P2-5)
    if !fees_percent.is_finite() || fee_rate < 0.0 {
        return Err(pyo3::exceptions::PyValueError::new_err("fee_rate must be finite and non-negative"));
    }

    // ✅ Match Python: (Kraken - Binance) / min_price
    let min_price = price_kraken.min(price_binance);
    let gross_spread = (price_kraken - price_binance) / min_price;
    
    // Net spread after fees
    let net = gross_spread - fees_percent;

    if !net.is_finite() {
        return Err(pyo3::exceptions::PyValueError::new_err("spread result is not finite"));
    }

    Ok(net)
}

/// Validates a price value (used by market data validators).
#[pyfunction]
fn validate_price(price: f64) -> PyResult<bool> {
    Ok(price.is_finite() && price > 0.0)
}
