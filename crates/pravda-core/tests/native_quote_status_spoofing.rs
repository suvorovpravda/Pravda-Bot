//! Adversarial PAPER-only tests: untrusted status strings cannot authenticate a quote.
//! Fixtures are synthetic, never exchange attestations or executable prices.
use pravda_core::agent_contract::{PaperSymbol, PaperVenue};
use pravda_core::agent_pipeline::{evidence_from_native_market_snapshot, NativeMarketRefusal};
use serde_json::json;

const NOW_MS: u64 = 1_762_768_800_000;

fn snapshot(status: serde_json::Value) -> serde_json::Value {
    json!({
        "stream_state": "observations_recent",
        "provenance_verified": true,
        "portfolio_verified": true,
        "symbols": {"BTC/USDT": {"Binance": {
            "status": status,
            "timestamp": "2025-11-10T10:00:00Z",
            "age_seconds": 0,
            "price": 100,
            "attested": true,
            "provenance_verified": true,
            "portfolio_verified": true
        }}}
    })
}

#[test]
fn native_quote_status_requires_exact_live_string() {
    for status in [
        json!("LIVE"),
        json!("live "),
        json!(" live"),
        json!("live\n"),
        json!("live\u{0000}"),
        json!("stale"),
        json!("cached"),
        json!(true),
        json!(1),
        serde_json::Value::Null,
    ] {
        let refusal = evidence_from_native_market_snapshot(
            &snapshot(status.clone()),
            PaperSymbol::BtcUsdt,
            PaperVenue::Binance,
            NOW_MS,
            2_000,
            50,
        ).expect_err("spoofed quote status must fail closed");
        assert_eq!(refusal, NativeMarketRefusal::QuoteStaleOrUntrusted, "{status}");
    }
}

#[test]
fn synthetically_live_status_is_not_market_attestation() {
    let evidence = evidence_from_native_market_snapshot(
        &snapshot(json!("live")),
        PaperSymbol::BtcUsdt,
        PaperVenue::Binance,
        NOW_MS,
        2_000,
        50,
    ).expect("synthetic observation may be parsed, never attested");
    assert!(!evidence.provenance_verified);
    assert!(!evidence.portfolio_verified);
}
