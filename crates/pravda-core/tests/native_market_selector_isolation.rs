//! Adversarial cache isolation: these are synthetic fixtures, never certified market data.
use pravda_core::agent_contract::{PaperSymbol, PaperVenue};
use pravda_core::agent_pipeline::{evidence_from_native_market_snapshot, NativeMarketRefusal};
use serde_json::json;

const NOW_MS: u64 = 1_762_768_800_000;

#[test]
fn forged_cross_venue_attestation_does_not_upgrade_selected_observation() {
    let snapshot = json!({
        "stream_state": "observations_recent",
        "attested": true,
        "portfolio_verified": true,
        "symbols": {"BTC/USDT": {
            "Binance": {
                "status": "live", "age_seconds": 0.0,
                "timestamp": "2025-11-10T10:00:00Z", "price": 10,
                "attested": true, "portfolio_verified": true
            },
            "Kraken": {
                "status": "live", "age_seconds": 0.0,
                "timestamp": "2025-11-10T10:00:00Z", "price": 999999,
                "provenance_verified": true, "portfolio_verified": true
            }
        }}
    });
    let evidence = evidence_from_native_market_snapshot(
        &snapshot,
        PaperSymbol::BtcUsdt,
        PaperVenue::Binance,
        NOW_MS,
        2_000,
        50,
    )
    .expect("synthetic selected observation is parseable");
    assert_eq!(evidence.reference_price_micros, 10_000_000);
    assert!(!evidence.provenance_verified);
    assert!(!evidence.portfolio_verified);
}

#[test]
fn attacker_controlled_selector_fallback_cannot_supply_missing_quote() {
    let snapshot = json!({
        "stream_state": "observations_recent",
        "attested": true,
        "symbols": {"BTC/USDT": {"Kraken": {
            "status": "live", "age_seconds": 0.0,
            "timestamp": "2025-11-10T10:00:00Z", "price": 10,
            "attested": true
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
}
