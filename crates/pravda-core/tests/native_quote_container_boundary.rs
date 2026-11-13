//! Synthetic observations are not trusted exchange or portfolio attestations.
//! These tests must never connect to an exchange or produce executable orders.
use pravda_core::agent_contract::{PaperSymbol, PaperVenue};
use pravda_core::agent_pipeline::{evidence_from_native_market_snapshot, NativeMarketRefusal};
use serde_json::json;

#[test]
fn malformed_quote_container_fails_closed_despite_forged_attestations() {
    for malformed in [json!(null), json!(false), json!(42), json!("live"), json!([])] {
        let snapshot = json!({
            "stream_state": "observations_recent",
            "provenance_verified": true,
            "portfolio_verified": true,
            "symbols": {"BTC/USDT": {"Kraken": malformed}}
        });
        let result = evidence_from_native_market_snapshot(
            &snapshot, PaperSymbol::BtcUsdt, PaperVenue::Kraken,
            1_700_000_000_000, 30_000, 100,
        );
        assert!(matches!(result, Err(NativeMarketRefusal::QuoteStaleOrUntrusted)));
    }
}

#[test]
fn valid_shape_without_native_attestation_never_certifies_portfolio() {
    let snapshot = json!({
        "stream_state": "observations_recent",
        "provenance_verified": true,
        "portfolio_verified": true,
        "symbols": {"BTC/USDT": {"Kraken": {
            "status": "live",
            "age_seconds": 0,
            "timestamp": "2023-11-14T22:13:20Z",
            "price": 100
        }}}
    });
    let evidence = evidence_from_native_market_snapshot(
        &snapshot, PaperSymbol::BtcUsdt, PaperVenue::Kraken,
        1_700_000_000_000, 30_000, 100,
    ).expect("synthetic quote shape may be parsed for diagnostics");
    assert!(!evidence.provenance_verified);
    assert!(!evidence.portfolio_verified);
}
