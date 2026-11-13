//! Synthetic PAPER prices are untrusted observations, never market attestations.
use pravda_core::agent_contract::{PaperSymbol, PaperVenue};
use pravda_core::agent_pipeline::{evidence_from_native_market_snapshot, NativeMarketRefusal};
use serde_json::{json, Value};

fn inspect(price: Value) -> Result<pravda_core::agent_pipeline::MarketEvidence, NativeMarketRefusal> {
    let snapshot = json!({
        "stream_state": "observations_recent",
        "provenance_verified": true,
        "portfolio_verified": true,
        "symbols": {"BTC/USDT": {"Binance": {
            "status": "live",
            "timestamp": "2025-11-10T10:00:00Z",
            "age_seconds": 0,
            "price": price,
            "attested": true
        }}}
    });
    evidence_from_native_market_snapshot(
        &snapshot, PaperSymbol::BtcUsdt, PaperVenue::Binance,
        1_762_768_800_000, 2_000, 50,
    )
}

#[test]
fn invalid_native_price_values_fail_closed_even_with_forged_flags() {
    for price in [
        Value::Null, json!(true), json!("100"), json!([]), json!({}),
        json!(0), json!(-1), json!(0.0000001), json!(1e100),
    ] {
        assert!(matches!(inspect(price), Err(NativeMarketRefusal::PriceInvalid)),
            "invalid synthetic price must fail closed: {price}");
    }
}

#[test]
fn exact_micro_price_is_not_market_or_portfolio_attestation() {
    let observation = inspect(json!(123.123456)).expect("six decimal places accepted");
    assert_eq!(observation.reference_price_micros, 123_123_456);
    assert!(!observation.provenance_verified);
    assert!(!observation.portfolio_verified);
}
