//! Validated market calculations for the Rust migration. Not wired to the bot.
#![forbid(unsafe_code)]

pub mod health;

/// Fee estimate expressed as a fraction: 0.0036 = 0.36%.
pub const DEFAULT_FEE_RATE: f64 = 0.0036;
/// Maximum gross spread expressed in percentage points.
pub const DEFAULT_MAX_SPREAD_PERCENT: f64 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketError {
    InvalidPrice,
    InvalidFee,
    InvalidLimit,
    NonFiniteSpread,
}

/// Validate a price supplied as text by a public feed.
pub fn parse_positive_price(raw: &str) -> Result<f64, MarketError> {
    let price: f64 = raw.trim().parse().map_err(|_| MarketError::InvalidPrice)?;
    if !price.is_finite() || price <= 0.0_f64 {
        return Err(MarketError::InvalidPrice);
    }
    Ok(price)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spread {
    pub gross_fraction: f64,
    pub net_fraction: f64,
}

impl Spread {
    pub fn gross_percent(self) -> f64 {
        self.gross_fraction * 100.0_f64
    }

    pub fn net_percent(self) -> f64 {
        self.net_fraction * 100.0_f64
    }
}

/// Absolute price spread, using the lower price as denominator, minus fees.
pub fn calculate_spread(first: f64, second: f64, fee_rate: f64) -> Result<Spread, MarketError> {
    if !first.is_finite() || first <= 0.0_f64 || !second.is_finite() || second <= 0.0_f64 {
        return Err(MarketError::InvalidPrice);
    }
    if !fee_rate.is_finite() || fee_rate < 0.0_f64 {
        return Err(MarketError::InvalidFee);
    }
    let gross: f64 = (first - second).abs() / first.min(second);
    let net: f64 = gross - fee_rate;
    if !gross.is_finite()
        || !net.is_finite()
        || !(gross * 100.0_f64).is_finite()
        || !(net * 100.0_f64).is_finite()
    {
        return Err(MarketError::NonFiniteSpread);
    }
    Ok(Spread {
        gross_fraction: gross,
        net_fraction: net,
    })
}

/// Check a gross spread measured in percentage points; zero cannot alert.
pub fn allowed_spread(gross_percent: f64, maximum_percent: f64) -> Result<bool, MarketError> {
    if !maximum_percent.is_finite() || maximum_percent <= 0.0_f64 {
        return Err(MarketError::InvalidLimit);
    }
    Ok(gross_percent.is_finite() && gross_percent > 0.0_f64 && gross_percent <= maximum_percent)
}

#[cfg(test)]
mod tests {
    use super::{
        allowed_spread, calculate_spread, parse_positive_price, MarketError, DEFAULT_FEE_RATE,
    };

    #[test]
    fn prices_from_text_must_be_finite_and_positive() {
        assert_eq!(parse_positive_price(" 100.25 "), Ok(100.25_f64));
        for raw in ["", "true", "NaN", "inf", "0", "-1", "1e999"] {
            assert_eq!(parse_positive_price(raw), Err(MarketError::InvalidPrice));
        }
    }

    #[test]
    fn fees_use_fraction_and_percentage_points_correctly() {
        let spread =
            calculate_spread(100.0_f64, 101.2_f64, DEFAULT_FEE_RATE).expect("valid prices");
        assert!((spread.gross_fraction - 0.012_f64).abs() < 1e-12_f64);
        assert!((spread.net_percent() - 0.84_f64).abs() < 1e-10_f64);
    }

    #[test]
    fn reversed_prices_produce_the_same_absolute_spread() {
        assert_eq!(
            calculate_spread(100.0_f64, 101.0_f64, 0.0_f64),
            calculate_spread(101.0_f64, 100.0_f64, 0.0_f64)
        );
    }

    #[test]
    fn invalid_prices_and_fees_are_rejected() {
        for price in [0.0_f64, -1.0_f64, f64::NAN, f64::INFINITY] {
            assert_eq!(
                calculate_spread(price, 1.0_f64, 0.0_f64),
                Err(MarketError::InvalidPrice)
            );
        }
        for fee in [-1.0_f64, f64::NAN, f64::INFINITY] {
            assert_eq!(
                calculate_spread(1.0_f64, 2.0_f64, fee),
                Err(MarketError::InvalidFee)
            );
        }
        assert_eq!(
            calculate_spread(f64::MIN_POSITIVE, f64::MAX, 0.0_f64),
            Err(MarketError::NonFiniteSpread)
        );
        assert_eq!(
            calculate_spread(1.0_f64, 1.0_f64, f64::MAX),
            Err(MarketError::NonFiniteSpread)
        );
    }

    #[test]
    fn guardrail_covers_boundary_and_invalid_inputs() {
        assert_eq!(allowed_spread(5.0_f64, 5.0_f64), Ok(true));
        assert_eq!(allowed_spread(5.01_f64, 5.0_f64), Ok(false));
        assert_eq!(allowed_spread(0.0_f64, 5.0_f64), Ok(false));
        assert_eq!(allowed_spread(f64::NAN, 5.0_f64), Ok(false));
        assert_eq!(
            allowed_spread(1.0_f64, f64::NAN),
            Err(MarketError::InvalidLimit)
        );
    }
}
