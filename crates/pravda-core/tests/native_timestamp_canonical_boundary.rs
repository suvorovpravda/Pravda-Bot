//! Native PAPER quote timestamps must be canonical millisecond UTC values.
//! Test fixtures are observations, never independent market or portfolio attestations.
use pravda_core::agent_contract::{PaperSymbol, PaperVenue};
use pravda_core::agent_pipeline::{evidence_from_native_market_snapshot, NativeMarketRefusal};
use serde_json::json;

const NOW: u64 = 1_762_768_800_000;

#[test]
fn noncanonical_utc_and_submillisecond_quotes_fail_closed() {
    for timestamp in [
        "2025-11-10T10:00:00.0001Z",
        "2025-11-10T10:00:00.000001Z",
        "2025-11-10T10:00:00+00:00",
        "2025-11-10T11:00:00+01:00",
    ] {
        let snapshot = json!({
            "stream_state": "observations_recent",
            "provenance_verified": true,
            "portfolio_verified": true,
            "symbols": { "BTC/USDT": { "Binance": {
                "status": "live",
                "timestamp": timestamp,
                "age_seconds": 0,
                "price": 100,
                "attested": true
            }}}
        });
        assert!(matches!(
            evidence_from_native_market_snapshot(
                &snapshot, PaperSymbol::BtcUsdt, PaperVenue::Binance, NOW, 2_000, 50
            ),
            Err(NativeMarketRefusal::ClockOrAgeInvalid)
        ), "noncanonical timestamp must fail closed: {timestamp}");
    }
}

#[test]
fn canonical_millisecond_quote_is_only_an_unattested_observation() {
    let snapshot = json!({
        "stream_state": "observations_recent",
        "provenance_verified": true,
        "portfolio_verified": true,
        "symbols": { "BTC/USDT": { "Binance": {
            "status": "live",
            "timestamp": "2025-11-10T10:00:00.000Z",
            "age_seconds": 0,
            "price": 100
        }}}
    });
    let evidence = evidence_from_native_market_snapshot(
        &snapshot, PaperSymbol::BtcUsdt, PaperVenue::Binance, NOW, 2_000, 50
    ).expect("canonical synthetic quote can be normalized");
    assert!(!evidence.provenance_verified);
    assert!(!evidence.portfolio_verified);
}
