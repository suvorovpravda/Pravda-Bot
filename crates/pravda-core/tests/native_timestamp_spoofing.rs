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
        assert!(matches!(
            evidence_from_native_market_snapshot(
                &snapshot, PaperSymbol::BtcUsdt, PaperVenue::Binance,
                NOW_MS, 2_000, 50
            ),
            Err(NativeMarketRefusal::ClockOrAgeInvalid)
        ), "noncanonical synthetic timestamp accepted: {timestamp:?}");
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
            &snapshot, PaperSymbol::BtcUsdt, PaperVenue::Binance,
            NOW_MS, 2_000, 50
        ),
        Err(NativeMarketRefusal::QuoteUnavailable)
    ));
    let evidence = evidence_from_native_market_snapshot(
        &snapshot, PaperSymbol::BtcUsdt, PaperVenue::Kraken,
        NOW_MS, 2_000, 50
    ).expect("synthetic observation should parse without attestation");
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
            &snapshot, PaperSymbol::BtcUsdt, PaperVenue::Binance,
            NOW_MS, 2_000, 50
        ),
        Err(NativeMarketRefusal::QuoteUnavailable)
    ));
    let evidence = evidence_from_native_market_snapshot(
        &snapshot, PaperSymbol::EthUsdt, PaperVenue::Binance,
        NOW_MS, 2_000, 50
    ).expect("synthetic ETH observation should parse without attestation");
    assert!(!evidence.provenance_verified);
    assert!(!evidence.portfolio_verified);
}
