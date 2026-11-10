//! Pure, deterministic REG contract for PAPER research only.
//! This module never authorizes brokerage execution, credentials, or a live API.
//! A valid risk review is NOT an executable order or a guaranteed stop fill.
//! All monetary fields are unsigned micro-USDT, with whole-asset units.
//! No model-generated prose influences these hard numeric predicates.

use crate::research_risk::FeedQuality;

const BASIS_POINTS: u128 = 10_000;
const ONE_PERCENT_BPS: u128 = 100;
const DAILY_LOSS_BPS: u128 = 300;
const WEEKLY_LOSS_BPS: u128 = 700;
const MAX_LEVERAGE_BPS: u128 = 30_000;
const SECTOR_LIMIT_BPS: u128 = 2_500;
const ASSET_LIMIT_BPS: u128 = 4_000;
const MARGIN_BUFFER_BPS: u128 = 12_000;
const MIN_REWARD_RISK_BPS: u128 = 20_000;
const MAX_CORRELATION_MILLI: u16 = 700;
const REARM_DELAY_SECONDS: u64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Paper,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Long,
    Short,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegRule {
    R001,
    R002,
    R003,
    R004,
    R005,
    R006,
    R007,
    R008,
    R009,
    R010,
    R011,
    R012,
}

impl RegRule {
    pub const fn code(self) -> &'static str {
        match self {
            Self::R001 => "R001",
            Self::R002 => "R002",
            Self::R003 => "R003",
            Self::R004 => "R004",
            Self::R005 => "R005",
            Self::R006 => "R006",
            Self::R007 => "R007",
            Self::R008 => "R008",
            Self::R009 => "R009",
            Self::R010 => "R010",
            Self::R011 => "R011",
            Self::R012 => "R012",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    ApprovedPaperOnly,
    Rejected,
    Escalated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingOrInvalid {
    LiveDisabled,
    InvalidPolicy,
    UnverifiedMarket,
    BadQuoteOrClock,
    MissingMarginOrFees,
    MissingCorrelation,
    InvalidNumerics,
}

#[derive(Debug, Clone, Copy)]
pub struct Policy {
    /// An explicitly approved maximum order size, never inferred from AI.
    pub max_position_micros: u128,
    pub max_quote_age_ms: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct Candidate {
    pub mode: Mode,
    pub side: Side,
    pub market_quality: FeedQuality,
    pub quote_received_at_ms: u64,
    pub now_ms: u64,
    pub entry_price_micros: u128,
    pub stop_price_micros: u128,
    pub target_price_micros: u128,
    pub underlying_units: u64,
    pub contract_multiplier: u64,
    pub fees_micros: Option<u128>,
    pub portfolio_equity_micros: u128,
    pub current_gross_micros: u128,
    pub daily_loss_micros: u128,
    pub weekly_loss_micros: u128,
    pub sector_exposure_micros: u128,
    pub asset_exposure_micros: u128,
    pub most_correlated_position_milli: Option<i16>,
    pub has_existing_positions: bool,
    pub available_margin_micros: Option<u128>,
    pub initial_margin_required_micros: Option<u128>,
    pub kill_switch_active: bool,
    pub human_rearm_confirmed: bool,
    pub last_emergency_stop_sec: Option<u64>,
    pub now_sec: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metrics {
    pub notional_micros: u128,
    pub risk_if_stopped_micros: u128,
    pub reward_after_fees_micros: u128,
    pub gross_exposure_after_micros: u128,
    pub daily_loss_if_stopped_micros: u128,
    pub weekly_loss_if_stopped_micros: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Review {
    pub verdict: Verdict,
    pub checks_performed: u8,
    pub violations: Vec<RegRule>,
    pub error: Option<MissingOrInvalid>,
    pub metrics: Option<Metrics>,
}

fn escalate(error: MissingOrInvalid) -> Review {
    Review {
        verdict: Verdict::Escalated,
        checks_performed: 0,
        violations: Vec::new(),
        error: Some(error),
        metrics: None,
    }
}

fn checked_sum(a: u128, b: u128) -> Result<u128, MissingOrInvalid> {
    a.checked_add(b).ok_or(MissingOrInvalid::InvalidNumerics)
}

fn checked_product(a: u128, b: u128) -> Result<u128, MissingOrInvalid> {
    a.checked_mul(b).ok_or(MissingOrInvalid::InvalidNumerics)
}

fn within_bps(amount: u128, equity: u128, limit_bps: u128) -> Result<bool, MissingOrInvalid> {
    let left = checked_product(amount, BASIS_POINTS)?;
    let right = checked_product(equity, limit_bps)?;
    Ok(left <= right)
}

fn calculate(policy: Policy, c: Candidate) -> Result<(Metrics, u128, u16), MissingOrInvalid> {
    if policy.max_position_micros == 0 || policy.max_quote_age_ms == 0 {
        return Err(MissingOrInvalid::InvalidPolicy);
    }
    if c.market_quality != FeedQuality::Verified {
        return Err(MissingOrInvalid::UnverifiedMarket);
    }
    if c.entry_price_micros == 0
        || c.target_price_micros == 0
        || c.underlying_units == 0
        || c.contract_multiplier == 0
        || c.portfolio_equity_micros == 0
    {
        return Err(MissingOrInvalid::BadQuoteOrClock);
    }
    let age = c
        .now_ms
        .checked_sub(c.quote_received_at_ms)
        .ok_or(MissingOrInvalid::BadQuoteOrClock)?;
    if age > policy.max_quote_age_ms {
        return Err(MissingOrInvalid::BadQuoteOrClock);
    }
    if c.last_emergency_stop_sec
        .is_some_and(|last| last > c.now_sec)
    {
        return Err(MissingOrInvalid::BadQuoteOrClock);
    }
    let fees = c.fees_micros.ok_or(MissingOrInvalid::MissingMarginOrFees)?;
    let initial_margin = c
        .initial_margin_required_micros
        .ok_or(MissingOrInvalid::MissingMarginOrFees)?;
    let available_margin = c
        .available_margin_micros
        .ok_or(MissingOrInvalid::MissingMarginOrFees)?;
    if initial_margin == 0 || available_margin > c.portfolio_equity_micros {
        return Err(MissingOrInvalid::InvalidNumerics);
    }
    let correlation = if c.has_existing_positions {
        let value = c
            .most_correlated_position_milli
            .ok_or(MissingOrInvalid::MissingCorrelation)?;
        if !(-1_000..=1_000).contains(&value) {
            return Err(MissingOrInvalid::InvalidNumerics);
        }
        value.unsigned_abs()
    } else {
        0
    };
    let units = checked_product(
        u128::from(c.underlying_units),
        u128::from(c.contract_multiplier),
    )?;
    let notional = checked_product(c.entry_price_micros, units)?;
    let stop_distance = c.entry_price_micros.abs_diff(c.stop_price_micros);
    // An upside target on a short (or downside target on a long) is a LOSS,
    // never a positive reward. Keep the reported reward at zero on that path.
    let target_distance = match c.side {
        Side::Long => c.target_price_micros.saturating_sub(c.entry_price_micros),
        Side::Short => c.entry_price_micros.saturating_sub(c.target_price_micros),
    };
    let risk = checked_sum(checked_product(stop_distance, units)?, fees)?;
    let gross_reward = checked_product(target_distance, units)?;
    let reward = gross_reward.saturating_sub(fees);
    let gross_after = checked_sum(c.current_gross_micros, notional)?;
    let daily_if_stopped = checked_sum(c.daily_loss_micros, risk)?;
    let weekly_if_stopped = checked_sum(c.weekly_loss_micros, risk)?;
    Ok((
        Metrics {
            notional_micros: notional,
            risk_if_stopped_micros: risk,
            reward_after_fees_micros: reward,
            gross_exposure_after_micros: gross_after,
            daily_loss_if_stopped_micros: daily_if_stopped,
            weekly_loss_if_stopped_micros: weekly_if_stopped,
        },
        checked_product(initial_margin, MARGIN_BUFFER_BPS)?
            .checked_add(BASIS_POINTS - 1)
            .ok_or(MissingOrInvalid::InvalidNumerics)?
            / BASIS_POINTS,
        correlation,
    ))
}

/// Only a deterministic paper assessment. Every new order must be checked by
/// the independently authenticated and persisted native kill switch as well.
pub fn evaluate(policy: Policy, c: Candidate) -> Review {
    if c.mode != Mode::Paper {
        return Review {
            verdict: Verdict::Rejected,
            checks_performed: 0,
            violations: Vec::new(),
            error: Some(MissingOrInvalid::LiveDisabled),
            metrics: None,
        };
    }
    let (metrics, required_margin_with_buffer, correlation) = match calculate(policy, c) {
        Ok(value) => value,
        Err(reason) => return escalate(reason),
    };
    let mut failures = Vec::new();
    let stop_on_correct_side = c.stop_price_micros != 0
        && match c.side {
            Side::Long => c.stop_price_micros < c.entry_price_micros,
            Side::Short => c.stop_price_micros > c.entry_price_micros,
        };
    if !stop_on_correct_side {
        failures.push(RegRule::R001);
    }
    let checks = (|| -> Result<(), MissingOrInvalid> {
        if !within_bps(
            metrics.risk_if_stopped_micros,
            c.portfolio_equity_micros,
            ONE_PERCENT_BPS,
        )? {
            failures.push(RegRule::R002);
        }
        let profitable_target = match c.side {
            Side::Long => c.target_price_micros > c.entry_price_micros,
            Side::Short => c.target_price_micros < c.entry_price_micros,
        };
        if !profitable_target
            || checked_product(metrics.reward_after_fees_micros, BASIS_POINTS)?
                < checked_product(metrics.risk_if_stopped_micros, MIN_REWARD_RISK_BPS)?
        {
            failures.push(RegRule::R003);
        }
        if metrics.notional_micros > policy.max_position_micros {
            failures.push(RegRule::R004);
        }
        if !within_bps(
            metrics.gross_exposure_after_micros,
            c.portfolio_equity_micros,
            MAX_LEVERAGE_BPS,
        )? {
            failures.push(RegRule::R005);
        }
        if !within_bps(
            metrics.daily_loss_if_stopped_micros,
            c.portfolio_equity_micros,
            DAILY_LOSS_BPS,
        )? {
            failures.push(RegRule::R006);
        }
        if !within_bps(
            metrics.weekly_loss_if_stopped_micros,
            c.portfolio_equity_micros,
            WEEKLY_LOSS_BPS,
        )? {
            failures.push(RegRule::R007);
        }
        if !within_bps(
            checked_sum(c.sector_exposure_micros, metrics.notional_micros)?,
            c.portfolio_equity_micros,
            SECTOR_LIMIT_BPS,
        )? {
            failures.push(RegRule::R008);
        }
        if correlation > MAX_CORRELATION_MILLI {
            failures.push(RegRule::R009);
        }
        if c.available_margin_micros.unwrap_or(0) < required_margin_with_buffer {
            failures.push(RegRule::R010);
        }
        if !within_bps(
            checked_sum(c.asset_exposure_micros, metrics.notional_micros)?,
            c.portfolio_equity_micros,
            ASSET_LIMIT_BPS,
        )? {
            failures.push(RegRule::R011);
        }
        let cooldown = c
            .last_emergency_stop_sec
            .and_then(|last| c.now_sec.checked_sub(last))
            .is_some_and(|elapsed| elapsed < REARM_DELAY_SECONDS);
        if c.kill_switch_active || !c.human_rearm_confirmed || cooldown {
            failures.push(RegRule::R012);
        }
        Ok(())
    })();
    if let Err(error) = checks {
        return escalate(error);
    }
    Review {
        verdict: if failures.is_empty() {
            Verdict::ApprovedPaperOnly
        } else {
            Verdict::Rejected
        },
        checks_performed: 12,
        violations: failures,
        error: None,
        metrics: Some(metrics),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> Policy {
        Policy {
            max_position_micros: 100_000_000,
            max_quote_age_ms: 2_000,
        }
    }

    fn candidate() -> Candidate {
        Candidate {
            mode: Mode::Paper,
            side: Side::Long,
            market_quality: FeedQuality::Verified,
            quote_received_at_ms: 9_500,
            now_ms: 10_000,
            entry_price_micros: 10_000_000,
            stop_price_micros: 9_500_000,
            target_price_micros: 11_500_000,
            underlying_units: 5,
            contract_multiplier: 1,
            fees_micros: Some(500_000),
            portfolio_equity_micros: 1_000_000_000,
            current_gross_micros: 100_000_000,
            daily_loss_micros: 10_000_000,
            weekly_loss_micros: 10_000_000,
            sector_exposure_micros: 100_000_000,
            asset_exposure_micros: 200_000_000,
            most_correlated_position_milli: Some(500),
            has_existing_positions: true,
            available_margin_micros: Some(100_000_000),
            initial_margin_required_micros: Some(50_000_000),
            kill_switch_active: false,
            human_rearm_confirmed: true,
            last_emergency_stop_sec: Some(1_000),
            now_sec: 1_500,
        }
    }

    fn must_reject(expected: RegRule, c: Candidate, policy: Policy) {
        let result = evaluate(policy, c);
        assert_eq!(result.verdict, Verdict::Rejected);
        assert_eq!(result.checks_performed, 12);
        assert!(
            result.violations.contains(&expected),
            "missing {} in {:?}",
            expected.code(),
            result.violations
        );
    }

    #[test]
    fn compliant_inputs_are_deterministic_paper_only() {
        let first = evaluate(policy(), candidate());
        assert_eq!(first, evaluate(policy(), candidate()));
        assert_eq!(first.verdict, Verdict::ApprovedPaperOnly);
        assert_eq!(first.checks_performed, 12);
        assert_eq!(first.metrics.unwrap().notional_micros, 50_000_000);
        assert_eq!(first.metrics.unwrap().risk_if_stopped_micros, 3_000_000);
        assert!(first.violations.is_empty());
    }

    #[test]
    fn every_r001_to_r012_rule_has_a_failing_fixture() {
        let mut c = candidate();
        c.stop_price_micros = 10_500_000;
        must_reject(RegRule::R001, c, policy());

        c = candidate();
        c.stop_price_micros = 8_000_000;
        must_reject(RegRule::R002, c, policy());

        c = candidate();
        c.target_price_micros = 10_100_000;
        must_reject(RegRule::R003, c, policy());

        let mut p = policy();
        p.max_position_micros = 40_000_000;
        must_reject(RegRule::R004, candidate(), p);

        c = candidate();
        c.current_gross_micros = 2_990_000_001;
        must_reject(RegRule::R005, c, policy());

        c = candidate();
        c.daily_loss_micros = 28_000_000;
        must_reject(RegRule::R006, c, policy());

        c = candidate();
        c.weekly_loss_micros = 68_000_000;
        must_reject(RegRule::R007, c, policy());

        c = candidate();
        c.sector_exposure_micros = 220_000_000;
        must_reject(RegRule::R008, c, policy());

        c = candidate();
        c.most_correlated_position_milli = Some(-701);
        must_reject(RegRule::R009, c, policy());

        c = candidate();
        c.available_margin_micros = Some(59_999_999);
        must_reject(RegRule::R010, c, policy());

        c = candidate();
        c.asset_exposure_micros = 355_000_000;
        must_reject(RegRule::R011, c, policy());

        c = candidate();
        c.kill_switch_active = true;
        must_reject(RegRule::R012, c, policy());
    }

    #[test]
    fn live_is_always_rejected_even_with_valid_numbers() {
        let mut c = candidate();
        c.mode = Mode::Live;
        let result = evaluate(policy(), c);
        assert_eq!(result.verdict, Verdict::Rejected);
        assert_eq!(result.error, Some(MissingOrInvalid::LiveDisabled));
    }

    #[test]
    fn missing_or_stale_market_and_portfolio_data_escalate() {
        let mut c = candidate();
        c.market_quality = FeedQuality::Stale;
        assert_eq!(evaluate(policy(), c).verdict, Verdict::Escalated);
        c = candidate();
        c.quote_received_at_ms = 7_000;
        assert_eq!(evaluate(policy(), c).verdict, Verdict::Escalated);
        c = candidate();
        c.most_correlated_position_milli = None;
        assert_eq!(
            evaluate(policy(), c).error,
            Some(MissingOrInvalid::MissingCorrelation)
        );
        c = candidate();
        c.fees_micros = None;
        assert_eq!(evaluate(policy(), c).verdict, Verdict::Escalated);
        c = candidate();
        c.available_margin_micros = None;
        assert_eq!(evaluate(policy(), c).verdict, Verdict::Escalated);
        c = candidate();
        c.last_emergency_stop_sec = Some(2_000);
        assert_eq!(evaluate(policy(), c).verdict, Verdict::Escalated);
    }

    #[test]
    fn overflow_and_bad_policy_escalate_without_a_partial_approval() {
        let mut c = candidate();
        c.underlying_units = u64::MAX;
        c.contract_multiplier = u64::MAX;
        c.entry_price_micros = u128::MAX;
        assert_eq!(evaluate(policy(), c).verdict, Verdict::Escalated);
        let mut p = policy();
        p.max_quote_age_ms = 0;
        assert_eq!(
            evaluate(p, candidate()).error,
            Some(MissingOrInvalid::InvalidPolicy)
        );
    }

    #[test]
    fn short_stop_must_be_above_entry_and_cooldown_persists() {
        let mut c = candidate();
        c.side = Side::Short;
        c.stop_price_micros = 10_500_000;
        c.target_price_micros = 8_500_000;
        assert_eq!(evaluate(policy(), c).verdict, Verdict::ApprovedPaperOnly);
        c.stop_price_micros = 9_500_000;
        must_reject(RegRule::R001, c, policy());
        c = candidate();
        c.now_sec = 1_001;
        must_reject(RegRule::R012, c, policy());
        c = candidate();
        c.human_rearm_confirmed = false;
        must_reject(RegRule::R012, c, policy());
    }

    #[test]
    fn no_existing_positions_do_not_require_a_correlation_matrix() {
        let mut c = candidate();
        c.has_existing_positions = false;
        c.most_correlated_position_milli = None;
        assert_eq!(evaluate(policy(), c).verdict, Verdict::ApprovedPaperOnly);
    }
    #[test]
    fn r003_never_counts_a_losing_or_flat_target_as_profit() {
        let mut long = candidate();
        long.target_price_micros = 8_500_000;
        let losing_long = evaluate(policy(), long);
        assert_eq!(losing_long.verdict, Verdict::Rejected);
        assert!(losing_long.violations.contains(&RegRule::R003));
        assert_eq!(losing_long.metrics.unwrap().reward_after_fees_micros, 0);

        long.target_price_micros = long.entry_price_micros;
        must_reject(RegRule::R003, long, policy());

        let mut short = candidate();
        short.side = Side::Short;
        short.stop_price_micros = 10_500_000;
        short.target_price_micros = 11_500_000;
        let losing_short = evaluate(policy(), short);
        assert_eq!(losing_short.verdict, Verdict::Rejected);
        assert!(losing_short.violations.contains(&RegRule::R003));
        assert_eq!(losing_short.metrics.unwrap().reward_after_fees_micros, 0);

        short.target_price_micros = short.entry_price_micros;
        must_reject(RegRule::R003, short, policy());
    }
}
