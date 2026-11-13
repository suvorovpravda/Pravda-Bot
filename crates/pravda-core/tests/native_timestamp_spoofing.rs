//! Adversarial timestamp normalization tests for the native PAPER boundary.
//! These synthetic fixtures are not certified market data or portfolio evidence.
use pravda_core::agent_contract::{PaperSymbol, PaperVenue};
use pravda_core::agent_pipeline::{evidence_from_native_market_snapshot, NativeMarketRefusal};
use serde_json::json;

const NOW_MS: u64 = 1_762_768_800_000;

#[test]
fn timestamp_unicode_and_whitespace_spoofing_fails_closed() {
    for timestamp in [
        "2025-11-10T10:00:00Z ",
        " 2025-11-10T10:00:00Z",
        "2025-11-10T10:00:00z",
        "2025-11-10T10:00:00Ｚ",
        "2025-11-10T10:00:00Z\n",
        "2025-11-10T10:00:00Z\0",
    ] {
        let snapshot = json!({
            "stream_state": "observations_recent",
            "symbols": { "BTC/USDT": { "Binance": {
                "status": "live",
                "age_seconds": 0.0,
                "timestamp": timestamp,
                "price": 10,
                "attested": true,
                "provenance_verified": true,
                "portfolio_verified": true
            }}}
        });
        assert!(
            matches!(
                evidence_from_native_market_snapshot(
                    &snapshot,
                    PaperSymbol::BtcUsdt,
                    PaperVenue::Binance,
                    NOW_MS,
                    2_000,
                    50
                ),
                Err(NativeMarketRefusal::ClockOrAgeInvalid)
            ),
            "noncanonical synthetic timestamp accepted: {timestamp:?}"
        );
    }
}

#[test]
fn quote_from_other_venue_cannot_satisfy_requested_native_market_pair() {
    // A fresh-looking quote for Kraken cannot be silently reused as Binance
    // evidence. Neither venue's synthetic fixture certifies provenance.
    let snapshot = json!({
        "stream_state": "observations_recent",
        "symbols": { "BTC/USDT": { "Kraken": {
            "status": "live",
            "age_seconds": 0.0,
            "timestamp": "2025-11-10T10:00:00Z",
            "price": 10,
            "provenance_verified": true,
            "portfolio_verified": true
        }}}
    });
    assert!(matches!(
        evidence_from_native_market_snapshot(
            &snapshot,
            PaperSymbol::BtcUsdt,
            PaperVenue::Binance,
            NOW_MS,
            2_000,
            50
        ),
        Err(NativeMarketRefusal::QuoteUnavailable)
    ));
    let evidence = evidence_from_native_market_snapshot(
        &snapshot,
        PaperSymbol::BtcUsdt,
        PaperVenue::Kraken,
        NOW_MS,
        2_000,
        50,
    )
    .expect("synthetic observation should parse without attestation");
    assert!(!evidence.provenance_verified);
    assert!(!evidence.portfolio_verified);
}

#[test]
fn quote_from_other_symbol_cannot_satisfy_requested_native_market_pair() {
    // A plausible ETH fixture cannot become BTC market evidence merely
    // because both symbols use the same venue and synthetic timestamp.
    let snapshot = json!({
        "stream_state": "observations_recent",
        "symbols": { "ETH/USDT": { "Binance": {
            "status": "live",
            "age_seconds": 0.0,
            "timestamp": "2025-11-10T10:00:00Z",
            "price": 10,
            "attested": true,
            "provenance_verified": true,
            "portfolio_verified": true
        }}}
    });
    assert!(matches!(
        evidence_from_native_market_snapshot(
            &snapshot,
            PaperSymbol::BtcUsdt,
            PaperVenue::Binance,
            NOW_MS,
            2_000,
            50
        ),
        Err(NativeMarketRefusal::QuoteUnavailable)
    ));
    let evidence = evidence_from_native_market_snapshot(
        &snapshot,
        PaperSymbol::EthUsdt,
        PaperVenue::Binance,
        NOW_MS,
        2_000,
        50,
    )
    .expect("synthetic ETH observation should parse without attestation");
    assert!(!evidence.provenance_verified);
    assert!(!evidence.portfolio_verified);
}

#[test]
fn embedded_identity_fields_cannot_override_native_quote_lookup() {
    // A quote object's self-declared identity is not an authoritative key.
    // Never fall back to embedded fields when the requested venue is absent.
    let snapshot = json!({
        "stream_state": "observations_recent",
        "symbols": { "BTC/USDT": { "Kraken": {
            "symbol": "BTC/USDT",
            "venue": "Binance",
            "status": "live",
            "age_seconds": 0.0,
            "timestamp": "2025-11-10T10:00:00Z",
            "price": 10,
            "attested": true,
            "provenance_verified": true,
            "portfolio_verified": true
        }}}
    });
    assert!(matches!(
        evidence_from_native_market_snapshot(
            &snapshot,
            PaperSymbol::BtcUsdt,
            PaperVenue::Binance,
            NOW_MS,
            2_000,
            50
        ),
        Err(NativeMarketRefusal::QuoteUnavailable)
    ));
    let observation = evidence_from_native_market_snapshot(
        &snapshot,
        PaperSymbol::BtcUsdt,
        PaperVenue::Kraken,
        NOW_MS,
        2_000,
        50,
    )
    .expect("synthetic quote is an observation, never an attestation");
    assert_eq!(observation.venue, PaperVenue::Kraken);
    assert!(!observation.provenance_verified);
    assert!(!observation.portfolio_verified);
}

#[test]
fn native_policy_zero_and_excessive_deviation_fail_before_quote_trust() {
    // A syntactically valid synthetic quote cannot override native PAPER
    // policy bounds, even when the cache forges independent trust flags.
    let snapshot = json!({
        "stream_state": "observations_recent",
        "attested": true,
        "portfolio_verified": true,
        "symbols": { "BTC/USDT": { "Binance": {
            "status": "live",
            "age_seconds": 0.0,
            "timestamp": "2025-11-10T10:00:00Z",
            "price": 10,
            "provenance_verified": true,
            "portfolio_verified": true
        }}}
    });
    for (max_age_ms, max_deviation_bps) in [
        (0_u64, 50_u16),
        (2_000, 0),
        (2_000, 1_001),
        (2_000, u16::MAX),
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
    // The maximum allowed deviation is still merely an observation.
    let observation = evidence_from_native_market_snapshot(
        &snapshot,
        PaperSymbol::BtcUsdt,
        PaperVenue::Binance,
        NOW_MS,
        2_000,
        1_000,
    )
    .expect("synthetic quote parses but is not certified");
    assert!(!observation.provenance_verified);
    assert!(!observation.portfolio_verified);
}

#[test]
fn forged_reported_age_cannot_override_native_wall_clock() {
    // A fresh synthetic timestamp and a stale reported age must disagree.
    // Neither field is an independent market or portfolio attestation.
    for reported_age in [2.0, 100.0, 1_000.0] {
        let snapshot = json!({
            "stream_state": "observations_recent",
            "symbols": { "BTC/USDT": { "Binance": {
                "status": "live",
                "age_seconds": reported_age,
                "timestamp": "2025-11-10T10:00:00Z",
                "price": 10,
                "attested": true,
                "provenance_verified": true,
                "portfolio_verified": true
            }}}
        });
        assert!(
            matches!(
                evidence_from_native_market_snapshot(
                    &snapshot,
                    PaperSymbol::BtcUsdt,
                    PaperVenue::Binance,
                    NOW_MS,
                    5_000_000,
                    50
                ),
                Err(NativeMarketRefusal::ClockOrAgeInvalid)
            ),
            "forged reported age accepted: {reported_age}"
        );
    }
}

#[test]
fn synthetic_prices_with_unsafe_numeric_representations_fail_closed() {
    // Market-cache JSON is untrusted: never round, coerce, or overflow a
    // synthetic price into an apparently usable PAPER reference price.
    for price in [
        json!(0),
        json!(-1),
        json!("10"),
        json!(null),
        json!(1.0000001),
        json!(1e30),
        json!(true),
    ] {
        let snapshot = json!({
            "stream_state": "observations_recent",
            "symbols": { "BTC/USDT": { "Binance": {
                "status": "live",
                "age_seconds": 0.0,
                "timestamp": "2025-11-10T10:00:00Z",
                "price": price,
                "attested": true,
                "provenance_verified": true,
                "portfolio_verified": true
            }}}
        });
        assert!(
            matches!(
                evidence_from_native_market_snapshot(
                    &snapshot,
                    PaperSymbol::BtcUsdt,
                    PaperVenue::Binance,
                    NOW_MS,
                    2_000,
                    50
                ),
                Err(NativeMarketRefusal::PriceInvalid)
            ),
            "unsafe synthetic price accepted: {price}"
        );
    }
}
