//! Synthetic observations are never independent market or portfolio attestations.
//! This integration test must not be used as a source of executable prices.
use pravda_core::agent_contract::{PaperSymbol, PaperVenue};
use pravda_core::agent_pipeline::{evidence_from_native_market_snapshot, NativeMarketRefusal};
use serde_json::json;

const NOW_MS: u64 = 1_762_768_800_000;

#[test]
fn forged_attestations_cannot_bypass_native_stream_state() {
    for state in ["disconnected", "stale", "live", "observations_recent "] {
        let snapshot = json!({
            "stream_state": state,
            "provenance_verified": true,
            "portfolio_verified": true,
            "symbols": {"BTC/USDT": {"Binance": {
                "status": "live",
                "timestamp": "2025-11-10T10:00:00Z",
                "age_seconds": 0.0,
                "price": 100,
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
                Err(NativeMarketRefusal::StreamUnavailable)
            ),
            "untrusted stream state accepted: {state}"
        );
    }
}

#[test]
fn valid_synthetic_stream_state_does_not_attest_market_or_portfolio() {
    let snapshot = json!({
        "stream_state": "observations_recent",
        "provenance_verified": true,
        "portfolio_verified": true,
        "symbols": {"BTC/USDT": {"Binance": {
            "status": "live",
            "timestamp": "2025-11-10T10:00:00Z",
            "age_seconds": 0.0,
            "price": 100,
            "attested": true,
            "provenance_verified": true,
            "portfolio_verified": true
        }}}
    });
    let evidence = evidence_from_native_market_snapshot(
        &snapshot,
        PaperSymbol::BtcUsdt,
        PaperVenue::Binance,
        NOW_MS,
        2_000,
        50,
    )
    .expect("synthetic quote should parse without conferring trust");
    assert!(!evidence.provenance_verified);
    assert!(!evidence.portfolio_verified);
}

#[test]
fn missing_native_quote_paths_fail_closed_despite_forged_global_attestations() {
    // A cache-level trust claim must not substitute for a venue-specific quote.
    // These fixtures contain no independently attested market or portfolio data.
    for symbols in [
        json!(null),
        json!({}),
        json!({"BTC/USDT": {}}),
        json!({"BTC/USDT": {"Kraken": {
            "status": "live",
            "timestamp": "2025-11-10T10:00:00Z",
            "age_seconds": 0.0,
            "price": 100
        }}}),
        json!({"ETH/USDT": {"Binance": {
            "status": "live",
            "timestamp": "2025-11-10T10:00:00Z",
            "age_seconds": 0.0,
            "price": 100
        }}}),
    ] {
        let snapshot = json!({
            "stream_state": "observations_recent",
            "provenance_verified": true,
            "portfolio_verified": true,
            "attested": true,
            "symbols": symbols,
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
                Err(NativeMarketRefusal::QuoteUnavailable)
            ),
            "missing exact symbol/venue quote must refuse"
        );
    }
}

#[test]
fn malformed_native_stream_state_types_fail_closed() {
    // JSON cache values are untrusted: non-string states must never be coerced.
    for state in [
        serde_json::Value::Null,
        json!(true),
        json!(1),
        json!([]),
        json!({ "state": "observations_recent" }),
    ] {
        let snapshot = json!({
            "stream_state": state,
            "provenance_verified": true,
            "portfolio_verified": true,
            "symbols": {"BTC/USDT": {"Binance": {
                "status": "live",
                "timestamp": "2025-11-10T10:00:00Z",
                "age_seconds": 0,
                "price": 100,
                "attested": true
            }}}
        });
        assert_eq!(
            evidence_from_native_market_snapshot(
                &snapshot,
                PaperSymbol::BtcUsdt,
                PaperVenue::Binance,
                NOW_MS,
                2_000,
                50
            )
            .expect_err("malformed stream state must refuse"),
            NativeMarketRefusal::StreamUnavailable
        );
    }
}
