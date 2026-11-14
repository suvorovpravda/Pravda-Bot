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
            Ok(evidence)
                if !evidence.provenance_verified
                    && !evidence.portfolio_verified
                    && evidence.symbol == symbol
                    && evidence.venue == venue
                    && evidence.reference_price_micros > 0
                    && evidence.quote_received_at_ms > 0
                    && evidence.quote_received_at_ms <= now_ms =>
            {
                "MDA_UNATTESTED_OBSERVATION"
            }
            // A future adapter must never silently turn a native cache quote
            // into independently attested evidence. Fail the entire IPC probe.
            Ok(_) => return Err("PAPER_UNEXPECTED_NATIVE_ATTESTATION"),
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
    fn cross_venue_or_symbol_cache_quotes_never_become_attested_evidence() {
        // A valid observation for one selector cannot be silently reused as
        // evidence for another symbol or venue, even with forged trust flags.
        for (symbol, venue) in [
            ("BTC/USDT", "Kraken"),
            ("ETH/USDT", "Binance"),
            ("ETH/USDT", "Kraken"),
            ("SOL/USDT", "Binance"),
            ("SOL/USDT", "Kraken"),
        ] {
            let rows = native_observation_probe(&observed(), symbol, venue, NOW_MS)
                .expect("missing selector must return a closed research diagnostic");
            assert!(rows.contains(&"MDA|MDA_QUOTE_UNAVAILABLE".to_owned()));
            for expected in [
                "market_attestation|ABSENT",
                "portfolio_attestation|ABSENT",
                "order_intent|NONE",
                "GMO|SKIPPED",
                "REG|SKIPPED",
                "portfolio|UNCHANGED",
                "execution|DISABLED",
            ] {
                assert!(rows.iter().any(|row| row == expected), "missing {expected}");
            }
        }
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
    fn malformed_quote_and_clock_fields_never_promote_native_market_evidence() {
        for (field, invalid) in [
            ("status", json!("verified")),
            ("age_seconds", json!(-0.0)),
            ("age_seconds", json!("0")),
            ("timestamp", json!("2026-11-10T10:00:00.0001Z")),
            ("timestamp", json!("2026-11-10T10:00:00+00:00")),
            ("price", json!(0)),
            ("price", json!("100.25")),
            ("price", json!(-1)),
        ] {
            let mut snapshot = observed();
            snapshot["symbols"]["BTC/USDT"]["Binance"][field] = invalid;
            snapshot["market_attestation"] = json!("VERIFIED");
            snapshot["portfolio_attestation"] = json!("VERIFIED");
            snapshot["proposal"] = json!({"side": "BUY", "quantity": 999999});
            let rows = native_observation_probe(&snapshot, "BTC/USDT", "Binance", NOW_MS)
                .expect("invalid quote fields must yield closed research diagnostics");
            assert!(
                rows.iter().any(|row| row.starts_with("MDA|MDA_"))
                    && !rows.contains(&"MDA|MDA_UNATTESTED_OBSERVATION".to_owned()),
                "invalid {field} must be refused by the MDA adapter"
            );
            for expected in [
                "verdict|BLOCKED_NO_INDEPENDENT_EVIDENCE",
                "market_attestation|ABSENT",
                "portfolio_attestation|ABSENT",
                "proposal|NONE",
                "order_intent|NONE",
                "GMO|SKIPPED",
                "REG|SKIPPED",
                "portfolio|UNCHANGED",
                "execution|DISABLED",
            ] {
                assert!(rows.iter().any(|row| row == expected), "missing {expected}");
            }
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
        assert!(!rows
            .iter()
            .any(|row| row.contains("APPROVED") || row.contains("ORDER_READY")));
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
        // UNVERIFIED is an intentional closed-state diagnostic; substring
        // matching VERIFIED would incorrectly reject that safe output.
        // Match only positive verdicts, not their negated status labels.
        assert!(!rows.iter().any(|row| row == "verdict|APPROVED"
            || row == "market_attestation|VERIFIED"
            || row == "portfolio_attestation|VERIFIED"
            || row == "proposal|ORDER_READY"
            || row == "execution|ENABLED"));
    }

    #[test]
    fn untrusted_symbols_venues_and_missing_native_clock_fail_closed() {
        for (symbol, venue, now, expected) in [
            ("DOGE/USDT", "Binance", NOW_MS, "PAPER_UNSUPPORTED_SYMBOL"),
            ("BTC/USDT", "FakeVenue", NOW_MS, "PAPER_UNSUPPORTED_VENUE"),
            ("BTC/USDT", "Binance", 0, "PAPER_CLOCK_UNAVAILABLE"),
        ] {
            assert_eq!(
                native_observation_probe(&observed(), symbol, venue, now),
                Err(expected)
            );
        }
    }

    #[test]
    fn selector_aliases_and_whitespace_cannot_bypass_native_allowlist() {
        // Selectors are exact Rust allowlist entries, not user-controlled
        // aliases or normalized identifiers from an untrusted IPC caller.
        for (symbol, venue, expected) in [
            ("btc/usdt", "Binance", "PAPER_UNSUPPORTED_SYMBOL"),
            (" BTC/USDT", "Binance", "PAPER_UNSUPPORTED_SYMBOL"),
            ("BTC/USDT ", "Binance", "PAPER_UNSUPPORTED_SYMBOL"),
            ("BTC/USDT", "binance", "PAPER_UNSUPPORTED_VENUE"),
            ("BTC/USDT", "Binance ", "PAPER_UNSUPPORTED_VENUE"),
            ("BTC/USDT", " Binance", "PAPER_UNSUPPORTED_VENUE"),
        ] {
            assert_eq!(
                native_observation_probe(&observed(), symbol, venue, NOW_MS),
                Err(expected),
                "noncanonical selector must fail before native market evaluation"
            );
        }
    }

    #[test]
    fn future_native_quote_clock_never_promotes_market_evidence() {
        // A source clock ahead of the trusted Rust clock is not fresh evidence.
        // Forged approval fields must not change the fail-closed diagnostic.
        let mut snapshot = observed();
        snapshot["market_attestation"] = json!("VERIFIED");
        snapshot["portfolio_attestation"] = json!("VERIFIED");
        snapshot["proposal"] = json!({"side": "BUY", "quantity": 1});
        let rows = native_observation_probe(&snapshot, "BTC/USDT", "Binance", NOW_MS - 1_000)
            .expect("future source clock must yield a closed research diagnostic");
        for expected in [
            "MDA|MDA_QUOTE_CLOCK_INVALID",
            "market_attestation|ABSENT",
            "portfolio_attestation|ABSENT",
            "proposal|NONE",
            "order_intent|NONE",
            "GMO|SKIPPED",
            "REG|SKIPPED",
            "portfolio|UNCHANGED",
            "execution|DISABLED",
        ] {
            assert!(rows.iter().any(|row| row == expected), "missing {expected}");
        }
    }

    #[test]
    fn stale_or_unavailable_quotes_cannot_authorize_a_paper_decision() {
        let mut snapshot = observed();
        snapshot["stream_state"] = json!("offline");
        let rows = native_observation_probe(&snapshot, "BTC/USDT", "Binance", NOW_MS).unwrap();
        assert!(rows.contains(&"MDA|MDA_STREAM_UNAVAILABLE".to_owned()));
        assert!(rows.contains(&"execution|DISABLED".to_owned()));
        let rows =
            native_observation_probe(&observed(), "BTC/USDT", "Binance", NOW_MS + 60_000).unwrap();
        assert!(rows.contains(&"MDA|MDA_QUOTE_CLOCK_INVALID".to_owned()));
        assert!(rows.contains(&"REG|SKIPPED".to_owned()));
    }
    #[test]
    fn paper_probe_emits_exactly_one_closed_diagnostic_per_role() {
        // A frontend must never interpret duplicated or missing role rows as
        // independent approvals. All seven roles remain non-executable.
        let rows = native_observation_probe(&observed(), "BTC/USDT", "Binance", NOW_MS)
            .expect("synthetic observation should produce a closed diagnostic");
        for role in ["ARCH", "SS", "MDA", "GMO", "REG", "IR", "UCA"] {
            assert_eq!(
                rows.iter()
                    .filter(|row| row.starts_with(&format!("{role}|")))
                    .count(),
                1,
                "role {role} must appear exactly once"
            );
        }
        assert_eq!(rows.len(), 15, "unexpected extra diagnostic fields");
        assert!(rows.contains(&"verdict|BLOCKED_NO_INDEPENDENT_EVIDENCE".to_owned()));
        assert!(rows.contains(&"order_intent|NONE".to_owned()));
        assert!(rows.contains(&"execution|DISABLED".to_owned()));
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
        assert!(!rows
            .iter()
            .any(|row| row.contains("LIVE") || row.contains("ORDER_READY")));
    }

    #[test]
    fn malformed_nested_quote_containers_never_reach_gmo_or_reg() {
        // A corrupted native cache shape must not be interpreted as an
        // independently verified market feed, even with forged approvals.
        for malformed in [json!(null), json!(true), json!(42), json!("BUY"), json!([])] {
            let mut snapshot = observed();
            snapshot["symbols"]["BTC/USDT"]["Binance"] = malformed;
            snapshot["market_attestation"] = json!("VERIFIED");
            snapshot["portfolio_attestation"] = json!("VERIFIED");
            snapshot["proposal"] = json!({"side": "BUY", "quantity": 1});
            let rows = native_observation_probe(&snapshot, "BTC/USDT", "Binance", NOW_MS)
                .expect("malformed nested quote must remain a blocked diagnostic");
            for expected in [
                "verdict|BLOCKED_NO_INDEPENDENT_EVIDENCE",
                "market_attestation|ABSENT",
                "portfolio_attestation|ABSENT",
                "proposal|NONE",
                "order_intent|NONE",
                "GMO|SKIPPED",
                "REG|SKIPPED",
                "portfolio|UNCHANGED",
                "execution|DISABLED",
            ] {
                assert!(rows.iter().any(|row| row == expected), "missing {expected}");
            }
            assert!(!rows.contains(&"MDA|MDA_UNATTESTED_OBSERVATION".to_owned()));
        }
    }
}
