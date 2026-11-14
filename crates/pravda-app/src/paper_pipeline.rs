//! Authenticated IPC uses this strictly read-only PAPER research probe.
//! A native market snapshot is an observation, never an independently
//! attested market feed or a verified portfolio. No GMO or REG is run here.
use pravda_core::agent_contract::{PaperSymbol, PaperVenue};
use pravda_core::agent_pipeline::evidence_from_native_market_snapshot;

/// Return closed, non-executable diagnostic codes. Neither JS selectors nor
/// source-provided attestation flags can make this a portfolio recommendation.
pub fn native_observation_probe(
    snapshot: &serde_json::Value,
    symbol: &str,
    venue: &str,
    now_ms: u64,
) -> Result<Vec<String>, &'static str> {
    let symbol = match symbol {
        "BTC/USDT" => PaperSymbol::BtcUsdt,
        "ETH/USDT" => PaperSymbol::EthUsdt,
        "SOL/USDT" => PaperSymbol::SolUsdt,
        _ => return Err("PAPER_UNSUPPORTED_SYMBOL"),
    };
    let venue = match venue {
        "Binance" => PaperVenue::Binance,
        "Kraken" => PaperVenue::Kraken,
        _ => return Err("PAPER_UNSUPPORTED_VENUE"),
    };
    if now_ms == 0 {
        return Err("PAPER_CLOCK_UNAVAILABLE");
    }
    // A malformed native snapshot is not a market observation. Refuse it
    // before passing data to the MDA adapter or reporting role diagnostics.
    if !snapshot.is_object() {
        return Err("PAPER_INVALID_NATIVE_SNAPSHOT");
    }

    // This checks structure, age, symbol, venue and numeric precision only.
    // The adapter deliberately sets both independent trust flags to false.
    let mda_code =
        match evidence_from_native_market_snapshot(snapshot, symbol, venue, now_ms, 5_000, 50) {
            Ok(evidence) if !evidence.provenance_verified && !evidence.portfolio_verified => {
                "MDA_UNATTESTED_OBSERVATION"
            }
            Ok(_) => "MDA_ATTESTATION_UNEXPECTED",
            Err(refusal) => refusal.code(),
        };

    // ARCH/SS have no independent attestation here. Subsequent roles are
    // explicitly skipped, not presented as having reviewed an order.
    Ok(vec![
        "mode|PAPER_RESEARCH_ONLY".to_owned(),
        "verdict|BLOCKED_NO_INDEPENDENT_EVIDENCE".to_owned(),
        // Explicit trust boundary for consumers: an observed price and a
        // forged JSON attestation never authorize risk or portfolio actions.
        "market_attestation|ABSENT".to_owned(),
        "portfolio_attestation|ABSENT".to_owned(),
        "proposal|NONE".to_owned(),
        // A research observation must not be mistaken for a portfolio order intent.
        "order_intent|NONE".to_owned(),
        "ARCH|UNVERIFIED".to_owned(),
        "SS|UNVERIFIED".to_owned(),
        format!("MDA|{mda_code}"),
        "GMO|SKIPPED".to_owned(),
        "REG|SKIPPED".to_owned(),
        "IR|HUMAN_REVIEW_REQUIRED".to_owned(),
        "UCA|UNATTESTED_DATA_ONLY".to_owned(),
        "portfolio|UNCHANGED".to_owned(),
        "execution|DISABLED".to_owned(),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const NOW_MS: u64 = 1_794_304_800_000;

    fn observed() -> serde_json::Value {
        json!({
            "stream_state": "observations_recent",
            "symbols": {
                "BTC/USDT": {
                    "Binance": {
                        "status": "live",
                        "age_seconds": 0,
                        "timestamp": "2026-11-10T10:00:00Z",
                        "price": 100.25,
                        "provenance_verified": true,
                        "portfolio_verified": true
                    }
                }
            },
            "provenance_verified": true,
            "portfolio_verified": true
        })
    }

    #[test]
    fn malformed_native_snapshot_fails_closed_before_any_role_diagnostic() {
        for invalid in [json!(null), json!([]), json!(true), json!(42), json!("BUY")] {
            assert_eq!(
                native_observation_probe(&invalid, "BTC/USDT", "Binance", NOW_MS),
                Err("PAPER_INVALID_NATIVE_SNAPSHOT")
            );
        }
    }

    #[test]
    fn forged_snapshot_attestations_never_enable_gmo_reg_or_orders() {
        let rows = native_observation_probe(&observed(), "BTC/USDT", "Binance", NOW_MS)
            .expect("valid synthetic observation must remain readable");
        assert!(rows.contains(&"verdict|BLOCKED_NO_INDEPENDENT_EVIDENCE".to_owned()));
        assert!(rows.contains(&"MDA|MDA_UNATTESTED_OBSERVATION".to_owned()));
        assert!(rows.contains(&"GMO|SKIPPED".to_owned()));
        assert!(rows.contains(&"REG|SKIPPED".to_owned()));
        assert!(rows.contains(&"portfolio|UNCHANGED".to_owned()));
        assert!(rows.contains(&"execution|DISABLED".to_owned()));
        assert!(!rows.iter().any(|row| row.contains("APPROVED") || row.contains("ORDER_READY")));
    }

    #[test]
    fn forged_nested_and_top_level_attestations_never_create_a_proposal() {
        let mut snapshot = observed();
        snapshot["symbols"]["BTC/USDT"]["Binance"]["attested"] = json!(true);
        snapshot["symbols"]["BTC/USDT"]["Binance"]["risk_approved"] = json!(true);
        snapshot["market_attestation"] = json!("VERIFIED");
        snapshot["portfolio_attestation"] = json!("VERIFIED");
        snapshot["proposal"] = json!({"side": "BUY", "quantity": 1000000});
        let rows = native_observation_probe(&snapshot, "BTC/USDT", "Binance", NOW_MS)
            .expect("forged claims must not promote the observed quote");
        for required in [
            "market_attestation|ABSENT",
            "portfolio_attestation|ABSENT",
            "proposal|NONE",
            "GMO|SKIPPED",
            "REG|SKIPPED",
            "portfolio|UNCHANGED",
            "execution|DISABLED",
        ] {
            assert!(rows.iter().any(|row| row == required), "missing {required}");
        }
        assert!(!rows.iter().any(|row| row.contains("APPROVED")
            || row.contains("ORDER_READY")
            || row.contains("VERIFIED")));
    }

    #[test]
    fn untrusted_symbols_venues_and_missing_native_clock_fail_closed() {
        for (symbol, venue, now, expected) in [
            ("DOGE/USDT", "Binance", NOW_MS, "PAPER_UNSUPPORTED_SYMBOL"),
            ("BTC/USDT", "FakeVenue", NOW_MS, "PAPER_UNSUPPORTED_VENUE"),
            ("BTC/USDT", "Binance", 0, "PAPER_CLOCK_UNAVAILABLE"),
        ] {
            assert_eq!(native_observation_probe(&observed(), symbol, venue, now), Err(expected));
        }
    }

    #[test]
    fn stale_or_unavailable_quotes_cannot_authorize_a_paper_decision() {
        let mut snapshot = observed();
        snapshot["stream_state"] = json!("offline");
        let rows = native_observation_probe(&snapshot, "BTC/USDT", "Binance", NOW_MS).unwrap();
        assert!(rows.contains(&"MDA|MDA_STREAM_UNAVAILABLE".to_owned()));
        assert!(rows.contains(&"execution|DISABLED".to_owned()));
        let rows = native_observation_probe(&observed(), "BTC/USDT", "Binance", NOW_MS + 60_000)
            .unwrap();
        assert!(rows.contains(&"MDA|MDA_QUOTE_CLOCK_INVALID".to_owned()));
        assert!(rows.contains(&"REG|SKIPPED".to_owned()));
    }
    #[test]
    fn forged_order_intent_and_live_mode_cannot_escape_read_only_research() {
        let mut snapshot = observed();
        snapshot["mode"] = json!("LIVE");
        snapshot["order_intent"] = json!({"side": "BUY", "symbol": "BTC/USDT"});
        snapshot["execution"] = json!("ENABLED");
        snapshot["symbols"]["BTC/USDT"]["Binance"]["order_intent"] = json!("APPROVED");
        let rows = native_observation_probe(&snapshot, "BTC/USDT", "Binance", NOW_MS)
            .expect("untrusted instructions must not change read-only diagnostics");
        for expected in [
            "mode|PAPER_RESEARCH_ONLY",
            "verdict|BLOCKED_NO_INDEPENDENT_EVIDENCE",
            "order_intent|NONE",
            "proposal|NONE",
            "GMO|SKIPPED",
            "REG|SKIPPED",
            "portfolio|UNCHANGED",
            "execution|DISABLED",
        ] {
            assert!(rows.iter().any(|row| row == expected), "missing {expected}");
        }
        assert!(!rows.iter().any(|row| row.contains("LIVE") || row.contains("ORDER_READY")));
    }

}
