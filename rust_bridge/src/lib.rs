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
    let gross = ((price_binance - price_kraken).abs() / ((price_kraken + price_binance) / 2.0)) * 100.0;
    let net = gross - fees_percent;
    Ok(net)
}

#[pyfunction]
fn validate_price(price: f64) -> PyResult<bool> {
    Ok(price.is_finite() && price > 0.0)
}
