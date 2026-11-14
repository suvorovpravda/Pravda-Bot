//! Adversarial PAPER-only clock boundaries. Synthetic quotes never attest a feed.
use pravda_core::agent_contract::{PaperSymbol, PaperVenue};
use pravda_core::agent_pipeline::{evidence_from_native_market_snapshot, NativeMarketRefusal};
use serde_json::json;

fn snapshot(timestamp: &str, age_seconds: f64) -> serde_json::Value {
    json!({
        "stream_state": "observations_recent",
        "provenance_verified": true,
        "portfolio_verified": true,
        "symbols": {"BTC/USDT": {"Binance": {
            "status": "live",
            "timestamp": timestamp,
            "age_seconds": age_seconds,
            "price": 100,
            "attested": true,
            "provenance_verified": true,
            "portfolio_verified": true
        }}}
    })
}

#[test]
fn native_quote_clock_rejects_future_and_epoch_overflow_even_with_forged_attestation() {
    const NOW: u64 = 1_762_768_800_000;
    for (timestamp, age) in [
        ("2025-11-10T10:00:00.001Z", 0.0),
        ("1969-12-31T23:59:59Z", 0.0),
        ("9999-12-31T23:59:59Z", 0.0),
    ] {
        let result = evidence_from_native_market_snapshot(
            &snapshot(timestamp, age),
            PaperSymbol::BtcUsdt,
            PaperVenue::Binance,
            NOW,
            2_000,
            50,
        );
        assert!(
            matches!(result, Err(NativeMarketRefusal::ClockOrAgeInvalid)),
            "invalid clock accepted: {timestamp}"
        );
    }
}

#[test]
fn native_quote_clock_rejects_reported_age_outside_policy_despite_fresh_timestamp() {
    const NOW: u64 = 1_762_768_800_000;
    let result = evidence_from_native_market_snapshot(
        &snapshot("2025-11-10T10:00:00Z", 2.001),
        PaperSymbol::BtcUsdt,
        PaperVenue::Binance,
        NOW,
        2_000,
        50,
    );
    assert!(matches!(
        result,
        Err(NativeMarketRefusal::ClockOrAgeInvalid)
    ));
}

#[test]
fn epoch_zero_quote_and_uninitialized_clock_fail_closed() {
    // A synthetic epoch-zero quote with a zero-age claim must not be
    // mistaken for fresh native evidence, even with forged trust flags.
    for now_ms in [0_u64, 1_u64] {
        let result = evidence_from_native_market_snapshot(
            &snapshot("1970-01-01T00:00:00Z", 0.0),
            PaperSymbol::BtcUsdt,
            PaperVenue::Binance,
            now_ms,
            2_000,
            50,
        );
        assert!(matches!(
            result,
            Err(NativeMarketRefusal::ClockOrAgeInvalid)
        ));
    }
}
