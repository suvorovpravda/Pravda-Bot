//! Offline, fail-closed Revolut X limit-order planning.
//!
//! Nothing in this module transmits, authorises, or signs an order. Live order
//! execution is deliberately absent pending account-specific authorisation,
//! risk review, vault provisioning, and tested exchange reconciliation.
//! The exchange request contract is documented at
//! https://developer.revolut.com/docs/api/revolut-x-crypto-exchange
use serde::{Deserialize, Serialize};
use thiserror::Error;

const SCALE: u128 = 100_000_000;
const MAX_DECIMAL_CHARS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimitOrderIntent {
    pub client_order_id: String,
    pub symbol: String,
    pub side: Side,
    pub base_size: String,
    pub price: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrderRiskLimits {
    /// Maximum total value of one order, in quote currency * 10^8.
    pub max_order_quote_units: u128,
    /// Remaining daily quote-currency budget, in quote currency * 10^8.
    pub remaining_daily_quote_units: u128,
    /// Unix millisecond timestamp of the last validated market observation.
    pub last_market_timestamp_ms: u64,
    /// Maximum permitted age of the last validated market observation.
    pub max_market_age_ms: u64,
    /// Last validated exchange reference price in quote currency.
    pub reference_price: &'static str,
    /// Absolute maximum price deviation from the reference, in basis points.
    pub max_price_deviation_bps: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum OrderIntentError {
    #[error("live trading is disabled")]
    TradingDisabled,
    #[error("invalid client order ID")]
    InvalidOrderId,
    #[error("instrument not on the allowlist")]
    InstrumentNotAllowed,
    #[error("invalid decimal quantity or price")]
    InvalidDecimal,
    #[error("market data is stale or from the future")]
    StaleMarket,
    #[error("risk limit is invalid or exceeded")]
    RiskLimit,
}

/// Parses a positive fixed-point quantity with at most eight fractional digits.
/// Floating-point numbers, exponent notation, signs and leading/trailing dots
/// are rejected so notional computations remain deterministic.
fn positive_units(value: &str) -> Option<u128> {
    if value.is_empty() || value.len() > MAX_DECIMAL_CHARS || !value.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return None;
    }
    let mut parts = value.split('.');
    let whole = parts.next()?;
    let fraction = parts.next();
    if whole.is_empty() || parts.next().is_some() {
        return None;
    }
    let whole: u128 = whole.parse().ok()?;
    let frac = match fraction {
        None => 0,
        Some(digits) if !digits.is_empty() && digits.len() <= 8 => {
            let parsed: u128 = digits.parse().ok()?;
            parsed.checked_mul(10_u128.pow(8 - digits.len() as u32))?
        }
        _ => return None,
    };
    let scaled = whole.checked_mul(SCALE)?.checked_add(frac)?;
    (scaled > 0).then_some(scaled)
}

fn is_uuid(value: &str) -> bool {
    if value.len() != 36 {
        return false;
    }
    value.bytes().enumerate().all(|(i, byte)| {
        if [8, 13, 18, 23].contains(&i) {
            byte == b'-'
        } else {
            byte.is_ascii_hexdigit()
        }
    })
}

/// Performs local validation only. No account, network or trading capability
/// can be reached from this function. The caller must separately enforce
/// persistent duplicate-ID prevention and exchange-side reconciliation.
pub fn validate_limit_order(
    intent: &LimitOrderIntent,
    limits: OrderRiskLimits,
    now_ms: u64,
    live_trading_enabled: bool,
) -> Result<u128, OrderIntentError> {
    if !live_trading_enabled {
        return Err(OrderIntentError::TradingDisabled);
    }
    if !is_uuid(&intent.client_order_id) {
        return Err(OrderIntentError::InvalidOrderId);
    }
    if !matches!(intent.symbol.as_str(), "BTC-USD" | "ETH-USD" | "SOL-USD") {
        return Err(OrderIntentError::InstrumentNotAllowed);
    }
    if limits.max_market_age_ms == 0
        || limits.last_market_timestamp_ms > now_ms
        || now_ms - limits.last_market_timestamp_ms > limits.max_market_age_ms
    {
        return Err(OrderIntentError::StaleMarket);
    }
    let size = positive_units(&intent.base_size).ok_or(OrderIntentError::InvalidDecimal)?;
    let price = positive_units(&intent.price).ok_or(OrderIntentError::InvalidDecimal)?;
    let reference =
        positive_units(limits.reference_price).ok_or(OrderIntentError::RiskLimit)?;
    if limits.max_price_deviation_bps == 0 || limits.max_price_deviation_bps > 10_000 {
        return Err(OrderIntentError::RiskLimit);
    }
    let deviation = price.abs_diff(reference);
    let actual = deviation
        .checked_mul(10_000)
        .ok_or(OrderIntentError::RiskLimit)?;
    let permitted = reference
        .checked_mul(u128::from(limits.max_price_deviation_bps))
        .ok_or(OrderIntentError::RiskLimit)?;
    if actual > permitted {
        return Err(OrderIntentError::RiskLimit);
    }
    // Round the quote notional UP, ensuring limits cannot be bypassed by
    // fractional remainders. Multiplication overflow fails closed.
    let product = size.checked_mul(price).ok_or(OrderIntentError::RiskLimit)?;
    let quote = product / SCALE + u128::from(product % SCALE != 0);
    if limits.max_order_quote_units == 0
        || limits.remaining_daily_quote_units == 0
        || quote > limits.max_order_quote_units
        || quote > limits.remaining_daily_quote_units
    {
        return Err(OrderIntentError::RiskLimit);
    }
    Ok(quote)
}

/// A validated, serialized order request for review and dry-run diagnostics.
/// It contains no API credential and cannot transmit itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedLimitOrder {
    pub body: String,
    pub quote_notional_units: u128,
}

/// Prepare the exact compact JSON used in the official signed POST example.
/// Keeping serialization in one place avoids signing bytes different from
/// those sent to the exchange when the execution layer is implemented.
pub fn prepare_limit_order(
    intent: &LimitOrderIntent,
    limits: OrderRiskLimits,
    now_ms: u64,
    live_trading_enabled: bool,
) -> Result<PreparedLimitOrder, OrderIntentError> {
    let quote_notional_units =
        validate_limit_order(intent, limits, now_ms, live_trading_enabled)?;
    let side = match intent.side {
        Side::Buy => "BUY",
        Side::Sell => "SELL",
    };
    let body = serde_json::json!({
        "client_order_id": intent.client_order_id,
        "symbol": intent.symbol,
        "side": side,
        "order_configuration": {
            "limit": {
                "base_size": intent.base_size,
                "price": intent.price
            }
        }
    })
    .to_string();
    Ok(PreparedLimitOrder {
        body,
        quote_notional_units,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn order() -> LimitOrderIntent {
        LimitOrderIntent {
            client_order_id: "3b364427-1f4f-4f66-9935-86b6fb115d26".into(),
            symbol: "BTC-USD".into(),
            side: Side::Buy,
            base_size: "0.01".into(),
            price: "90000".into(),
        }
    }
    fn limits() -> OrderRiskLimits {
        OrderRiskLimits {
            max_order_quote_units: 1000 * SCALE,
            remaining_daily_quote_units: 2000 * SCALE,
            last_market_timestamp_ms: 999_000,
            max_market_age_ms: 2_000,
            reference_price: "90000",
            max_price_deviation_bps: 100,
        }
    }
    #[test]
    fn disabled_by_default_even_for_valid_orders() {
        assert_eq!(validate_limit_order(&order(), limits(), 1_000_000, false), Err(OrderIntentError::TradingDisabled));
    }
    #[test]
    fn validates_fixed_point_notional_without_float_rounding() {
        assert_eq!(validate_limit_order(&order(), limits(), 1_000_000, true), Ok(900 * SCALE));
        assert_eq!(positive_units("0.00000001"), Some(1));
        for s in ["-1", "1e10", "NaN", ".1", "1.", "1.2.3", "0", "0.000000001", "1,5", ""] {
            assert_eq!(positive_units(s), None, "{s}");
        }
        assert_eq!(positive_units("99999999999999999999999999999999"), None);
    }
    #[test]
    fn rejects_bad_ids_symbols_age_and_risk_limits() {
        let mut o = order();
        o.client_order_id = "not-an-id".into();
        assert_eq!(validate_limit_order(&o, limits(), 1_000_000, true), Err(OrderIntentError::InvalidOrderId));
        o = order();
        o.symbol = "BTC-EUR".into();
        assert_eq!(validate_limit_order(&o, limits(), 1_000_000, true), Err(OrderIntentError::InstrumentNotAllowed));
        assert_eq!(validate_limit_order(&order(), limits(), 1_002_000, true), Err(OrderIntentError::StaleMarket));
        assert_eq!(validate_limit_order(&order(), limits(), 998_999, true), Err(OrderIntentError::StaleMarket));
        let mut l = limits();
        l.max_order_quote_units = 899 * SCALE;
        assert_eq!(validate_limit_order(&order(), l, 1_000_000, true), Err(OrderIntentError::RiskLimit));
        l = limits();
        l.remaining_daily_quote_units = 899 * SCALE;
        assert_eq!(validate_limit_order(&order(), l, 1_000_000, true), Err(OrderIntentError::RiskLimit));
    }
    #[test]
    fn price_collar_rejects_large_deviation_on_both_sides() {
        let mut o = order();
        o.price = "91800".into();
        assert_eq!(validate_limit_order(&o, limits(), 1_000_000, true), Err(OrderIntentError::RiskLimit));
        o.price = "88200".into();
        assert_eq!(validate_limit_order(&o, limits(), 1_000_000, true), Err(OrderIntentError::RiskLimit));
        o.price = "90001".into();
        assert_eq!(validate_limit_order(&o, limits(), 1_000_000, true), Ok(900_01000000));
        let mut l = limits();
        l.max_price_deviation_bps = 0;
        assert_eq!(validate_limit_order(&order(), l, 1_000_000, true), Err(OrderIntentError::RiskLimit));
    }

    #[test]
    fn preparing_order_is_local_and_requires_explicit_activation() {
        assert_eq!(
            prepare_limit_order(&order(), limits(), 1_000_000, false),
            Err(OrderIntentError::TradingDisabled)
        );
        let prepared = prepare_limit_order(&order(), limits(), 1_000_000, true).unwrap();
        let body: serde_json::Value = serde_json::from_str(&prepared.body).unwrap();
        assert_eq!(body["symbol"], "BTC-USD");
        assert_eq!(body["side"], "BUY");
        assert_eq!(body["order_configuration"]["limit"]["base_size"], "0.01");
        assert_eq!(body["order_configuration"]["limit"]["price"], "90000");
        assert_eq!(prepared.quote_notional_units, 900 * SCALE);
        assert!(body.get("api_key").is_none());
    }

    #[test]
    fn fractional_notional_rounds_up_and_cannot_bypass_ceiling() {
        let mut o = order();
        o.base_size = "0.00000001".into();
        o.price = "0.00000001".into();
        let mut l = limits();
        l.max_order_quote_units = 1;
        l.reference_price = "0.00000001";
        assert_eq!(validate_limit_order(&o, l, 1_000_000, true), Ok(1));
        l.max_order_quote_units = 0;
        assert_eq!(validate_limit_order(&o, l, 1_000_000, true), Err(OrderIntentError::RiskLimit));
    }
}
