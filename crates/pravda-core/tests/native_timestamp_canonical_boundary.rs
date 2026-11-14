//! Native PAPER quote timestamps must identify an exact, fresh instant.
//! Test fixtures are observations, never independent market or portfolio attestations.
use pravda_core::agent_contract::{PaperSymbol, PaperVenue};
use pravda_core::agent_pipeline::{evidence_from_native_market_snapshot, NativeMarketRefusal};
use serde_json::json;

const NOW: u64 = 1_762_768_800_000;

fn snapshot(timestamp: &str) -> serde_json::Value {
    json!({
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
    })
}

fn inspect(snapshot: &serde_json::Value) -> Result<pravda_core::agent_pipeline::MarketEvidence, NativeMarketRefusal> {
    evidence_from_native_market_snapshot(
        snapshot,
        PaperSymbol::BtcUsdt,
        PaperVenue::Binance,
        NOW,
        2_000,
        50,
    )
}

#[test]
fn known_rfc3339_offsets_are_accepted_without_attestation() {
    for timestamp in [
        "2025-11-10T10:00:00+00:00",
        "2025-11-10T11:00:00+01:00",
    ] {
        let evidence = inspect(&snapshot(timestamp))
            .expect("known RFC3339 offset for a fresh instant should parse");
        assert!(!evidence.provenance_verified);
        assert!(!evidence.portfolio_verified);
    }
}

#[test]
fn unknown_offset_and_submillisecond_future_quotes_fail_closed() {
    for timestamp in [
        "2025-11-10T10:00:00-00:00",
        "2025-11-10T10:00:00.0001Z",
        "2025-11-10T10:00:00.000001Z",
        "2025-11-10T10:00:00.000000001Z",
    ] {
        assert!(
            matches!(
                inspect(&snapshot(timestamp)),
                Err(NativeMarketRefusal::ClockOrAgeInvalid)
            ),
            "timestamp must fail closed: {timestamp}"
        );
    }
}

#[test]
fn canonical_millisecond_quote_is_only_an_unattested_observation() {
    let evidence = inspect(&snapshot("2025-11-10T10:00:00.000Z"))
        .expect("canonical synthetic quote can be normalized");
    assert!(!evidence.provenance_verified);
    assert!(!evidence.portfolio_verified);
}
