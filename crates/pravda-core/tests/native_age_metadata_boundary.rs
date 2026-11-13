//! Native PAPER quote age is untrusted cache metadata, never an attestation.
//! All fixtures are synthetic; no exchange, broker, or portfolio is connected.
use pravda_core::agent_contract::{PaperSymbol, PaperVenue};
use pravda_core::agent_pipeline::{evidence_from_native_market_snapshot, NativeMarketRefusal};
use serde_json::{json, Value};

fn inspect(age: Value) -> Result<pravda_core::agent_pipeline::MarketEvidence, NativeMarketRefusal> {
    let snapshot = json!({
        "stream_state": "observations_recent",
        "provenance_verified": true,
        "portfolio_verified": true,
        "symbols": {"BTC/USDT": {"Binance": {
            "status": "live",
            "timestamp": "2025-11-10T10:00:00Z",
            "age_seconds": age,
            "price": 100,
            "attested": true,
            "portfolio_verified": true
        }}}
    });
    evidence_from_native_market_snapshot(
        &snapshot,
        PaperSymbol::BtcUsdt,
        PaperVenue::Binance,
        1_762_768_800_000,
        2_000,
        50,
    )
}

#[test]
fn malformed_native_age_metadata_cannot_claim_quote_freshness() {
    for age in [
        Value::Null,
        json!(true),
        json!("0"),
        json!([]),
        json!({}),
        json!(-0.001),
        json!(2.001),
        json!(1e100),
    ] {
        assert!(
            matches!(inspect(age), Err(NativeMarketRefusal::ClockOrAgeInvalid)),
            "malformed synthetic age must fail closed"
        );
    }
}

#[test]
fn synthetically_fresh_age_never_attests_market_or_portfolio() {
    for age in [json!(0), json!(0.0), json!(1.0)] {
        let evidence = inspect(age).expect("synthetic age may parse as observation");
        assert!(!evidence.provenance_verified);
        assert!(!evidence.portfolio_verified);
    }
}
