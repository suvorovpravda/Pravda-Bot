use pyo3::prelude::*;
use pyo3::types::PyFloat;

#[pymodule]
fn pravda_rust_bridge(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(calculate_net_spread, m)?)?;
    m.add_function(wrap_pyfunction!(validate_price, m)?)?;
    Ok(())
}

#[pyfunction]
fn calculate_net_spread(price_kraken: f64, price_binance: f64, fees_percent: f64) -> PyResult<f64> {
    if price_kraken <= 0.0 || price_binance <= 0.0 {
        return Err(pyo3::exceptions::PyValueError::new_err("prices must be positive"));
    }
    // Match Python security_and_fees.calculate_net_spread_safe formula
    // Uses minimum price as denominator, no *100 multiplier
    let min_price = price_kraken.min(price_binance);
    let gross = (price_binance - price_kraken).abs() / min_price - fees_percent;
    let net = gross;
    Ok(net)
}

#[pyfunction]
fn validate_price(price: f64) -> PyResult<bool> {
    Ok(price.is_finite() && price > 0.0)
}
