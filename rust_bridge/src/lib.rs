use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Calcule le spread brut et net entre Kraken et Binance.
#[pyfunction]
fn calculate_spread(price_kraken: f64, price_binance: f64, fee_rate: f64) -> PyResult<(f64, f64)> {
    if !price_kraken.is_finite() || !price_binance.is_finite() || price_kraken <= 0.0 || price_binance <= 0.0 {
        return Err(PyValueError::new_err("prices must be finite and positive"));
    }
    if !fee_rate.is_finite() || fee_rate < 0.0 {
        return Err(PyValueError::new_err("fee_rate must be finite and non-negative"));
    }

    let gross = (price_kraken - price_binance) / price_binance;
    let net = gross - fee_rate;

    if !net.is_finite() {
        return Err(PyValueError::new_err("spread result is not finite"));
    }

    Ok((gross, net))
}

/// Valide qu'un prix est fini et positif.
#[pyfunction]
fn validate_price(price: f64) -> PyResult<bool> {
    Ok(price.is_finite() && price > 0.0)
}

/// INITIALIZER PYO3 pour Python import
#[pymodule]
fn pravda_rust_bridge(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(calculate_spread, m)?)?;
    m.add_function(wrap_pyfunction!(validate_price, m)?)?;
    Ok(())
}
