//! A native cache is an observation source, not an exchange or portfolio attestor.
//! Cross-venue and cross-symbol substitution must fail closed before REG.
use pravda_core::agent_contract::{PaperSymbol, PaperVenue};
use pravda_core::agent_pipeline::{evidence_from_native_market_snapshot, NativeMarketRefusal};
use serde_json::json;

const NOW: u64 = 1_762_768_800_000;

#[test]
fn native_market_evidence_is_bound_to_requested_symbol_and_venue() {
    let snapshot = json!({
        "stream_state": "observations_recent",
        "provenance_verified": true,
        "portfolio_verified": true,
        "symbols": {
            "BTC/USDT": {
                "Binance": {
                    "status": "live",
                    "timestamp": "2025-11-10T10:00:00Z",
                    "age_seconds": 0,
                    "price": 100,
                    "attested": true,
                    "portfolio_verified": true
                }
            }
        }
    });
    for (symbol, venue) in [
        (PaperSymbol::EthUsdt, PaperVenue::Binance),
        (PaperSymbol::SolUsdt, PaperVenue::Binance),
        (PaperSymbol::BtcUsdt, PaperVenue::Kraken),
        (PaperSymbol::EthUsdt, PaperVenue::Kraken),
    ] {
        assert!(matches!(
            evidence_from_native_market_snapshot(&snapshot, symbol, venue, NOW, 2_000, 50),
            Err(NativeMarketRefusal::QuoteUnavailable)
        ), "a different symbol or venue must never inherit the BTC/Binance quote");
    }
    let evidence = evidence_from_native_market_snapshot(
        &snapshot, PaperSymbol::BtcUsdt, PaperVenue::Binance, NOW, 2_000, 50,
    ).expect("synthetic observation can be normalized");
    assert_eq!(evidence.reference_price_micros, 100_000_000);
    assert!(!evidence.provenance_verified);
    assert!(!evidence.portfolio_verified);
}
