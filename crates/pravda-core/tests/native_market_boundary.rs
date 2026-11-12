//! Black-box boundary checks for the native PAPER market adapter.
//! Synthetic quotes below are test fixtures, not independently attested prices.
use pravda_core::agent_contract::{PaperSymbol, PaperVenue};
use pravda_core::agent_pipeline::{evidence_from_native_market_snapshot, NativeMarketRefusal};
use serde_json::json;

const NOW_MS: u64 = 1_762_768_800_000;

fn quote(price: serde_json::Value) -> serde_json::Value {
    json!({
        "stream_state": "observations_recent",
        "symbols": {
            "BTC/USDT": {
                "Binance": {
                    "status": "live",
                    "age_seconds": 0.0,
                    "timestamp": "2025-11-10T10:00:00Z",
                    "price": price,
                    "provenance_verified": true,
                    "portfolio_verified": true,
                    "attested": true
                }
            }
        }
    })
}

fn inspect(
    snapshot: &serde_json::Value,
) -> Result<pravda_core::agent_pipeline::MarketEvidence, NativeMarketRefusal> {
    evidence_from_native_market_snapshot(
        snapshot,
        PaperSymbol::BtcUsdt,
        PaperVenue::Binance,
        NOW_MS,
        2_000,
        50,
    )
}

#[test]
fn synthetic_quote_cannot_self_certify_market_or_portfolio() {
    // An exact observed timestamp still does not constitute feed attestation.
    let evidence = inspect(&quote(json!(10))).expect("well-formed observation");
    assert!(!evidence.provenance_verified);
    assert!(!evidence.portfolio_verified);
    assert_eq!(evidence.reference_price_micros, 10_000_000);
}

#[test]
fn malformed_or_non_numeric_prices_fail_closed() {
    for price in [
        json!("10"),
        json!(-1),
        json!(0),
        json!(null),
        json!({"value": 10, "attested": true}),
        json!(10.1234567),
    ] {
        assert!(matches!(
            inspect(&quote(price)),
            Err(NativeMarketRefusal::PriceInvalid)
        ));
    }
}

#[test]
fn venue_substitution_and_clock_regression_fail_closed() {
    let snapshot = quote(json!(10));
    assert!(matches!(
        evidence_from_native_market_snapshot(
            &snapshot,
            PaperSymbol::BtcUsdt,
            PaperVenue::Kraken,
            NOW_MS,
            2_000,
            50
        ),
        Err(NativeMarketRefusal::QuoteUnavailable)
    ));
    assert!(matches!(
        evidence_from_native_market_snapshot(
            &snapshot,
            PaperSymbol::BtcUsdt,
            PaperVenue::Binance,
            NOW_MS - 1,
            2_000,
            50
        ),
        Err(NativeMarketRefusal::ClockOrAgeInvalid)
    ));
}

#[test]
fn missing_or_unparseable_quote_timestamp_never_creates_market_evidence() {
    for timestamp in [
        json!(null),
        json!(""),
        json!("not-a-timestamp"),
        json!("9999-12-31T23:59:59Z"),
    ] {
        let mut snapshot = quote(json!(10));
        snapshot["symbols"]["BTC/USDT"]["Binance"]["timestamp"] = timestamp;
        assert!(matches!(
            inspect(&snapshot),
            Err(NativeMarketRefusal::ClockOrAgeInvalid)
        ));
    }
}

#[test]
fn untrusted_age_metadata_cannot_override_quote_wall_clock() {
    // Synthetic fixture: timestamp is exactly NOW_MS, so a claimed age of
    // 1.5 seconds conflicts with the independent timestamp beyond tolerance.
    let mut snapshot = quote(json!(10));
    snapshot["symbols"]["BTC/USDT"]["Binance"]["age_seconds"] = json!(1.5);
    assert!(matches!(
        inspect(&snapshot),
        Err(NativeMarketRefusal::ClockOrAgeInvalid)
    ));

    // A negative or nonnumeric age cannot become a trusted fresh quote.
    for age in [json!(-0.1), json!("0"), json!(null)] {
        snapshot["symbols"]["BTC/USDT"]["Binance"]["age_seconds"] = age;
        assert!(matches!(
            inspect(&snapshot),
            Err(NativeMarketRefusal::ClockOrAgeInvalid)
        ));
    }
}

#[test]
fn stale_stream_and_quote_status_refuse_synthetic_attestation_flags() {
    let mut snapshot = quote(json!(10));
    snapshot["stream_state"] = json!("disconnected");
    assert!(matches!(
        inspect(&snapshot),
        Err(NativeMarketRefusal::StreamUnavailable)
    ));
    snapshot["stream_state"] = json!("observations_recent");
    snapshot["symbols"]["BTC/USDT"]["Binance"]["status"] = json!("stale");
    assert!(matches!(
        inspect(&snapshot),
        Err(NativeMarketRefusal::QuoteStaleOrUntrusted)
    ));
}

#[test]
fn invalid_native_quote_policy_never_produces_market_evidence() {
    // Even a syntactically valid synthetic quote cannot bypass invalid
    // native risk limits. Neither successful parsing nor fixture metadata
    // constitutes independent market or portfolio attestation.
    let snapshot = quote(json!(10));
    for (max_age_ms, max_deviation_bps) in [
        (0, 50),
        (2_000, 0),
        (2_000, 1_001),
        (0, 0),
    ] {
        assert!(matches!(
            evidence_from_native_market_snapshot(
                &snapshot,
                PaperSymbol::BtcUsdt,
                PaperVenue::Binance,
                NOW_MS,
                max_age_ms,
                max_deviation_bps,
            ),
            Err(NativeMarketRefusal::PolicyInvalid)
        ));
    }
}
