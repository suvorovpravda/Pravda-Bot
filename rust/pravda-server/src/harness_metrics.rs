//! Strict, versioned, offline performance telemetry and fail-closed gates.
//! Money is in USD cents; drawdown/VaR in basis points; latency in microseconds.
//! Ratios must be calculated by the deterministic replay, never from log text.
//! Sharpe/Sortino/Calmar are per-run ratios, NOT recomputed by aggregation.
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metrics {
    pub schema_version: u16,
    pub run_id: String,
    pub gross_pnl_cents: i64,
    pub fees_cents: u64,
    pub net_pnl_cents: i64,
    pub max_drawdown_bps: u32,
    pub sharpe: f64,
    pub sortino: f64,
    pub calmar: f64,
    pub wins: u64,
    pub losses: u64,
    pub trades: u64,
    pub slippage_bps: f64,
    pub latency_p50_us: u64,
    pub latency_p99_us: u64,
    pub var_99_bps: u32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateLimits {
    pub min_trades: u64,
    pub min_net_pnl_cents: i64,
    pub min_sharpe: f64,
    pub min_sortino: f64,
    pub min_calmar: f64,
    pub max_drawdown_bps: u32,
    pub max_var_99_bps: u32,
    pub max_slippage_bps: f64,
    pub max_latency_p99_us: u64,
}

impl Default for GateLimits {
    fn default() -> Self {
        // Provisional acceptance thresholds. Must be signed off against the
        // desk's risk mandate before any production promotion.
        Self {
            min_trades: 200,
            min_net_pnl_cents: 0,
            min_sharpe: 0.0,
            min_sortino: 0.0,
            min_calmar: 0.0,
            max_drawdown_bps: 1_500, // 15%
            max_var_99_bps: 400,     // 4%
            max_slippage_bps: 15.0,
            max_latency_p99_us: 25_000, // 25 ms
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BuildError {
    #[error("invalid_schema")]
    InvalidSchema,
    #[error("invalid_run_id")]
    InvalidRunId,
    #[error("nonfinite_or_inconsistent_metrics")]
    InvalidMetrics,
    #[error("invalid_gate_limits")]
    InvalidLimits,
    #[error("insufficient_samples")]
    InsufficientSamples,
    #[error("net_pnl_below_minimum")]
    NetPnl,
    #[error("max_drawdown_exceeded")]
    Drawdown,
    #[error("var_exceeded")]
    ValueAtRisk,
    #[error("risk_adjusted_return_below_minimum")]
    RiskAdjustedReturn,
    #[error("slippage_exceeded")]
    Slippage,
    #[error("p99_latency_exceeded")]
    Latency,
}

/// Fixed institutional acceptance function requested by the client.
pub fn validate_build(metrics: &Metrics) -> Result<(), BuildError> {
    validate_with_limits(metrics, &GateLimits::default())
}

/// Configurable variant for risk-approved signed acceptance profiles.
pub fn validate_with_limits(m: &Metrics, limits: &GateLimits) -> Result<(), BuildError> {
    if m.schema_version != SCHEMA_VERSION {
        return Err(BuildError::InvalidSchema);
    }
    if m.run_id.is_empty()
        || m.run_id.len() > 64
        || !m
            .run_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
    {
        return Err(BuildError::InvalidRunId);
    }
    if ![m.sharpe, m.sortino, m.calmar, m.slippage_bps]
        .iter()
        .all(|n| n.is_finite())
        || m.slippage_bps < 0.0
        || m.latency_p50_us > m.latency_p99_us
        || m.wins.checked_add(m.losses).is_none_or(|x| x > m.trades)
        || i128::from(m.gross_pnl_cents) - i128::from(m.fees_cents)
            != i128::from(m.net_pnl_cents)
        || m.max_drawdown_bps > 10_000
        || m.var_99_bps > 10_000
    {
        return Err(BuildError::InvalidMetrics);
    }
    if ![limits.min_sharpe, limits.min_sortino, limits.min_calmar, limits.max_slippage_bps]
        .iter()
        .all(|n| n.is_finite())
        || limits.max_slippage_bps < 0.0
        || limits.min_trades == 0
        || limits.max_latency_p99_us == 0
        || limits.max_drawdown_bps > 10_000
        || limits.max_var_99_bps > 10_000
    {
        return Err(BuildError::InvalidLimits);
    }
    if m.trades < limits.min_trades {
        return Err(BuildError::InsufficientSamples);
    }
    if m.net_pnl_cents < limits.min_net_pnl_cents {
        return Err(BuildError::NetPnl);
    }
    if m.max_drawdown_bps > limits.max_drawdown_bps {
        return Err(BuildError::Drawdown);
    }
    if m.var_99_bps > limits.max_var_99_bps {
        return Err(BuildError::ValueAtRisk);
    }
    if m.sharpe < limits.min_sharpe
        || m.sortino < limits.min_sortino
        || m.calmar < limits.min_calmar
    {
        return Err(BuildError::RiskAdjustedReturn);
    }
    if m.slippage_bps > limits.max_slippage_bps {
        return Err(BuildError::Slippage);
    }
    if m.latency_p99_us > limits.max_latency_p99_us {
        return Err(BuildError::Latency);
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Aggregate {
    pub runs: u32,
    pub pass: u32,
    pub rejected: u32,
    pub timeout: u32,
    pub crash: u32,
    pub invalid: u32,
    pub gross_pnl_cents_total: i64,
    pub net_pnl_cents_total: i64,
    pub worst_max_drawdown_bps: u32,
    pub worst_var_99_bps: u32,
    pub worst_latency_p99_us: u64,
    pub mean_latency_p99_us: f64,
    pub mean_sharpe: f64,
    pub mean_sortino: f64,
    pub mean_calmar: f64,
    pub mean_slippage_bps: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub schema_version: u16,
    pub build_accepted: bool,
    pub aggregate: Aggregate,
    pub rejected_reasons: Vec<ReasonCount>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReasonCount {
    pub code: String,
    pub count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn example() -> Metrics {
        Metrics {
            schema_version: 1,
            run_id: "offline_fixture_1".into(),
            gross_pnl_cents: 12_000,
            fees_cents: 2_000,
            net_pnl_cents: 10_000,
            max_drawdown_bps: 250,
            sharpe: 1.2,
            sortino: 1.4,
            calmar: 1.1,
            wins: 120,
            losses: 80,
            trades: 200,
            slippage_bps: 2.0,
            latency_p50_us: 500,
            latency_p99_us: 2_000,
            var_99_bps: 120,
        }
    }

    #[test]
    fn rejects_unknown_telemetry_fields_and_invalid_floats() {
        let mut value = serde_json::to_value(example()).unwrap();
        value["inject_command"] = serde_json::json!("ignored?");
        assert!(serde_json::from_value::<Metrics>(value).is_err());

        let mut bad = example();
        bad.sharpe = f64::NAN;
        assert_eq!(validate_build(&bad), Err(BuildError::InvalidMetrics));
    }

    #[test]
    fn gates_are_fail_closed_and_check_financial_and_technical_limits() {
        let m = example();
        assert_eq!(validate_build(&m), Ok(()));
        let mut bad = m.clone();
        bad.max_drawdown_bps = 1_501;
        assert_eq!(validate_build(&bad), Err(BuildError::Drawdown));
        bad = m.clone();
        bad.latency_p99_us = 25_001;
        assert_eq!(validate_build(&bad), Err(BuildError::Latency));
        bad = m.clone();
        bad.net_pnl_cents = 10_001;
        assert_eq!(validate_build(&bad), Err(BuildError::InvalidMetrics));
        bad = m;
        bad.trades = 10;
        assert_eq!(validate_build(&bad), Err(BuildError::InsufficientSamples));
    }
}
