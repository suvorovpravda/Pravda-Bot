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

fn inspect(
    snapshot: &serde_json::Value,
) -> Result<pravda_core::agent_pipeline::MarketEvidence, NativeMarketRefusal> {
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
    for timestamp in ["2025-11-10T10:00:00+00:00", "2025-11-10T11:00:00+01:00"] {
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

#[test]
fn quote_freshness_limit_is_inclusive_at_exact_millisecond_precision() {
    // Reported age and wall-clock age agree exactly; only the strict
    // 2_000 ms policy boundary differs between these observations.
    let mut at_limit = snapshot("2025-11-10T09:59:58.000Z");
    at_limit["symbols"]["BTC/USDT"]["Binance"]["age_seconds"] = json!(2.0);
    let evidence = inspect(&at_limit).expect("exactly 2_000 ms is within policy");
    assert!(!evidence.provenance_verified);
    assert!(!evidence.portfolio_verified);

    let mut past_limit = snapshot("2025-11-10T09:59:57.999Z");
    past_limit["symbols"]["BTC/USDT"]["Binance"]["age_seconds"] = json!(2.001);
    assert!(matches!(
        inspect(&past_limit),
        Err(NativeMarketRefusal::ClockOrAgeInvalid)
    ));
}

#[test]
fn submillisecond_freshness_boundary_does_not_round_stale_quotes_into_policy() {
    // A rounded millisecond DTO would make both observations appear 2_000 ms old.
    // The native adapter must enforce the limit at the source's nanosecond precision.
    let mut just_inside = snapshot("2025-11-10T09:59:58.000000001Z");
    just_inside["symbols"]["BTC/USDT"]["Binance"]["age_seconds"] = json!(2.0);
    let evidence = inspect(&just_inside).expect("one nanosecond inside policy");
    assert!(!evidence.provenance_verified);
    assert!(!evidence.portfolio_verified);

    let mut just_outside = snapshot("2025-11-10T09:59:57.999999999Z");
    just_outside["symbols"]["BTC/USDT"]["Binance"]["age_seconds"] = json!(2.0);
    assert!(matches!(
        inspect(&just_outside),
        Err(NativeMarketRefusal::ClockOrAgeInvalid)
    ));
}

#[test]
fn reported_age_tolerance_cannot_override_wall_clock_freshness() {
    // The reported age is attacker-controlled cache metadata. At a 2s policy
    // limit, a quote older than 2s is refused even if its claimed age is zero.
    let mut stale = snapshot("2025-11-10T09:59:57.999Z");
    stale["symbols"]["BTC/USDT"]["Binance"]["age_seconds"] = json!(0);
    assert!(matches!(
        inspect(&stale),
        Err(NativeMarketRefusal::ClockOrAgeInvalid)
    ));

    // A quote inside the policy window is still refused if its reported age
    // disagrees with the native wall clock by more than the 1s tolerance.
    let mut inconsistent = snapshot("2025-11-10T09:59:58.999Z");
    inconsistent["symbols"]["BTC/USDT"]["Binance"]["age_seconds"] = json!(0);
    assert!(matches!(
        inspect(&inconsistent),
        Err(NativeMarketRefusal::ClockOrAgeInvalid)
    ));

    // Exactly 1s of disagreement is tolerated, without attesting provenance.
    let mut boundary = snapshot("2025-11-10T09:59:59.000Z");
    boundary["symbols"]["BTC/USDT"]["Binance"]["age_seconds"] = json!(0);
    let evidence = inspect(&boundary).expect("exact tolerance remains an observation");
    assert!(!evidence.provenance_verified);
    assert!(!evidence.portfolio_verified);
}

#[test]
fn reported_fractional_age_above_policy_limit_fails_without_rounding() {
    // The native wall clock is exactly at the policy limit. A cache-reported
    // age even one microsecond beyond it must not be rounded back into policy.
    let mut quote = snapshot("2025-11-10T09:59:58.000Z");
    quote["symbols"]["BTC/USDT"]["Binance"]["age_seconds"] = json!(2.000001);
    assert!(matches!(
        inspect(&quote),
        Err(NativeMarketRefusal::ClockOrAgeInvalid)
    ));

    // The inclusive boundary remains an observation, never an attestation.
    quote["symbols"]["BTC/USDT"]["Binance"]["age_seconds"] = json!(2.0);
    let evidence = inspect(&quote).expect("exact age limit remains admissible");
    assert!(!evidence.provenance_verified);
    assert!(!evidence.portfolio_verified);
}

#[test]
fn equivalent_timezone_offsets_preserve_the_same_unattested_instant() {
    // Offset boundaries must be normalized as instants, not compared as text.
    // These are synthetic observations, never independently attested prices.
    for timestamp in [
        "2025-11-10T10:00:00Z",
        "2025-11-10T00:00:00-10:00",
        "2025-11-11T00:00:00+14:00",
        "2025-11-10T10:00:00+00:00",
    ] {
        let evidence = inspect(&snapshot(timestamp))
            .expect("equivalent RFC3339 offset must identify the same fresh instant");
        assert_eq!(evidence.quote_received_at_ms, NOW, "{timestamp}");
        assert!(!evidence.provenance_verified);
        assert!(!evidence.portfolio_verified);
    }
}
