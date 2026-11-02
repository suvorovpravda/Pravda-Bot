//! Behavioral parity against the archived Python implementation.
//!
//! Sources:
//! - archive/python-legacy/tests/test_paper_simulator.py
//! - archive/python-legacy/tests/test_security_and_fees.py
//!
//! Monetary values are compared at the 8-decimal precision of the legacy
//! paper_trades.csv writer. Actions, position counts and completion counts
//! are compared exactly. No Python code is executed at test time.

use chrono::{DateTime, Utc};
use pravda_engine::{PaperMode, PaperSimulator};
use pravda_market_core::{
    allowed_spread, calculate_spread, parse_positive_price, DEFAULT_FEE_RATE,
};
use serde_json::Value;

fn field_number(value: &Value, key: &str) -> f64 {
    value[key]
        .as_f64()
        .unwrap_or_else(|| panic!("missing numeric fixture field: {key}"))
}

fn assert_money_matches_python(actual: f64, expected: f64, context: &str) {
    assert!(
        actual.is_finite(),
        "{context}: expected a finite monetary value, got {actual}"
    );
    assert_eq!(
        format!("{actual:.8}"),
        format!("{expected:.8}"),
        "{context}: 8-decimal Python CSV monetary parity"
    );
}

#[test]
fn parity_tests_legacy_paper_simulator() {
    let fixtures: Value = serde_json::from_str(include_str!("fixtures/parity_cases.json"))
        .expect("valid JSON fixture");
    let scenarios = fixtures["cases"].as_array().expect("scenario array");
    assert_eq!(scenarios.len(), 5, "all five legacy scenarios must execute");

    for scenario in scenarios {
        let name = scenario["name"].as_str().expect("scenario name");
        let mode = match scenario["mode"].as_str().expect("paper mode") {
            "portfolio" => PaperMode::Portfolio,
            "fixed" => PaperMode::Fixed,
            other => panic!("{name}: unexpected mode {other}"),
        };
        // These are precisely the fixture settings used by the archived Python tests.
        let mut sim = PaperSimulator::new(mode, 1000.0, 10.0, 100.0, 0.05, 3600.0, 0.36);
        sim.cash_usdt = field_number(scenario, "initial_cash");

        for (index, step) in scenario["steps"]
            .as_array()
            .expect("scenario steps")
            .iter()
            .enumerate()
        {
            let context = format!("{name} step {index}");
            let symbol = step["symbol"].as_str().expect("symbol");
            let now = DateTime::parse_from_rfc3339(step["at"].as_str().expect("RFC3339 timestamp"))
                .expect("valid timestamp")
                .with_timezone(&Utc);
            let action = sim.update_with_execution_approval(
                symbol,
                field_number(step, "kraken"),
                field_number(step, "binance"),
                step["confirmed"].as_bool().expect("confirmation flag"),
                step["execution_approved"]
                    .as_bool()
                    .expect("execution guard flag"),
                now,
            );

            assert_eq!(
                action.as_deref(),
                step["action"].as_str(),
                "{context}: action parity"
            );
            if let Some(notional) = step["notional"].as_f64() {
                let position = sim.positions.get(symbol).expect("position opened");
                assert_money_matches_python(
                    position.notional_usdt,
                    notional,
                    &format!("{context}: notional"),
                );
            }
            assert_money_matches_python(
                sim.cash_usdt,
                field_number(step, "cash"),
                &format!("{context}: cash"),
            );
            assert_money_matches_python(
                sim.realized_pnl_usdt,
                field_number(step, "realized_pnl"),
                &format!("{context}: realized P&L"),
            );
            assert_eq!(
                sim.completed_positions,
                step["completed"].as_u64().expect("completed positions"),
                "{context}: completed position count"
            );
            assert_eq!(
                sim.positions.len(),
                step["open_positions"].as_u64().expect("open positions") as usize,
                "{context}: open position count"
            );
        }
    }
}

#[test]
fn parity_tests_legacy_security_and_fees() {
    let fixtures: Value = serde_json::from_str(include_str!("fixtures/parity_cases.json"))
        .expect("valid JSON fixture");

    for fee_case in fixtures["security_cases"]
        .as_array()
        .expect("fee scenarios")
    {
        let price_1 = field_number(fee_case, "first_price");
        let price_2 = field_number(fee_case, "second_price");
        let fee = field_number(fee_case, "fee_rate");
        let result = calculate_spread(price_1, price_2, fee).expect("legacy-valid fee inputs");
        assert_money_matches_python(
            result.net_fraction,
            field_number(fee_case, "expected_net_rate"),
            "fee rate fraction",
        );
    }

    assert_eq!(DEFAULT_FEE_RATE, 0.0036, "0.36% equals fractional 0.0036");
    assert_eq!(parse_positive_price("123.45").unwrap(), 123.45);
    for invalid in ["not-a-price", "NaN", "inf", "0", "-1", "1e999"] {
        assert!(
            parse_positive_price(invalid).is_err(),
            "reject legacy-invalid {invalid}"
        );
    }
    assert_eq!(allowed_spread(0.01, 5.0), Ok(true));
    assert_eq!(allowed_spread(5.0, 5.0), Ok(true));
    assert_eq!(allowed_spread(0.0, 5.0), Ok(false));
    assert_eq!(allowed_spread(-0.1, 5.0), Ok(false));
    assert_eq!(allowed_spread(5.01, 5.0), Ok(false));
}
