//! Pure, deterministic *paper-research* risk gate. No API, network, secrets,
//! brokerage permissions, clock access or execution side effects.
//!
//! This is an independently testable proposed contract, NOT wired into the
//! running paper simulator and NOT permission to submit real orders.
//! All monetary fields use USDT micro-units (1 USDT = 1_000_000 units);
//! this type must never be mixed with BTC satoshis or venue-native quantities.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UsdtMicros(pub u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedQuality {
    Verified,
    Unknown,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResearchRiskPolicy {
    /// Must be explicitly enabled only in an isolated paper/research context.
    pub paper_enabled: bool,
    pub max_order_notional: UsdtMicros,
    pub max_total_exposure: UsdtMicros,
    pub max_realized_daily_loss: UsdtMicros,
    pub max_quote_age_ms: u64,
    pub max_spread_basis_points: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaperCandidate {
    pub order_notional: UsdtMicros,
    pub existing_exposure: UsdtMicros,
    pub realized_daily_loss: UsdtMicros,
    /// Milliseconds from a caller-supplied logical/replay time source.
    pub quote_received_at_ms: u64,
    pub now_ms: u64,
    pub price_usdt_micros: UsdtMicros,
    pub spread_basis_points: u32,
    pub feed_quality: FeedQuality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResearchRiskRefusal {
    PaperDisabled,
    InvalidLimits,
    UnverifiedFeed,
    MissingPrice,
    ClockRegression,
    StaleQuote,
    ExcessiveSpread,
    DailyLossLimit,
    OrderLimit,
    ExposureOverflow,
    ExposureLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaperRiskAssessment {
    pub exposure_after: UsdtMicros,
}

/// A *paper-only* predicate, deliberately independent from any OMS or venue.
///
/// This function never creates an order or writes any state. A pass means that
/// this limited collection of checks passed, NOT that execution is permitted,
/// that a strategy is profitable, or that the whole security model is valid.
pub fn evaluate_paper_risk(
    policy: ResearchRiskPolicy,
    request: PaperCandidate,
) -> Result<PaperRiskAssessment, ResearchRiskRefusal> {
    if !policy.paper_enabled {
        return Err(ResearchRiskRefusal::PaperDisabled);
    }
    if policy.max_order_notional.0 == 0
        || policy.max_total_exposure.0 == 0
        || policy.max_realized_daily_loss.0 == 0
        || policy.max_quote_age_ms == 0
        || policy.max_spread_basis_points == 0
        || policy.max_order_notional > policy.max_total_exposure
    {
        return Err(ResearchRiskRefusal::InvalidLimits);
    }
    if request.feed_quality != FeedQuality::Verified {
        return Err(ResearchRiskRefusal::UnverifiedFeed);
    }
    if request.price_usdt_micros.0 == 0 {
        return Err(ResearchRiskRefusal::MissingPrice);
    }
    let Some(age) = request.now_ms.checked_sub(request.quote_received_at_ms) else {
        return Err(ResearchRiskRefusal::ClockRegression);
    };
    if age > policy.max_quote_age_ms {
        return Err(ResearchRiskRefusal::StaleQuote);
    }
    if request.spread_basis_points > policy.max_spread_basis_points {
        return Err(ResearchRiskRefusal::ExcessiveSpread);
    }
    if request.realized_daily_loss >= policy.max_realized_daily_loss {
        return Err(ResearchRiskRefusal::DailyLossLimit);
    }
    if request.order_notional.0 == 0 || request.order_notional > policy.max_order_notional {
        return Err(ResearchRiskRefusal::OrderLimit);
    }
    let Some(total) = request
        .existing_exposure
        .0
        .checked_add(request.order_notional.0)
    else {
        return Err(ResearchRiskRefusal::ExposureOverflow);
    };
    if total > policy.max_total_exposure.0 {
        return Err(ResearchRiskRefusal::ExposureLimit);
    }
    Ok(PaperRiskAssessment {
        exposure_after: UsdtMicros(total),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> ResearchRiskPolicy {
        ResearchRiskPolicy {
            paper_enabled: true,
            max_order_notional: UsdtMicros(50_000_000),
            max_total_exposure: UsdtMicros(150_000_000),
            max_realized_daily_loss: UsdtMicros(20_000_000),
            max_quote_age_ms: 4_000,
            max_spread_basis_points: 50,
        }
    }

    fn candidate() -> PaperCandidate {
        PaperCandidate {
            order_notional: UsdtMicros(25_000_000),
            existing_exposure: UsdtMicros(50_000_000),
            realized_daily_loss: UsdtMicros(0),
            quote_received_at_ms: 9_000,
            now_ms: 10_000,
            price_usdt_micros: UsdtMicros(100_000_000),
            spread_basis_points: 10,
            feed_quality: FeedQuality::Verified,
        }
    }

    #[test]
    fn conservative_default_and_missing_feed_cannot_pass() {
        let mut p = policy();
        p.paper_enabled = false;
        assert_eq!(
            evaluate_paper_risk(p, candidate()),
            Err(ResearchRiskRefusal::PaperDisabled)
        );
        let mut c = candidate();
        c.feed_quality = FeedQuality::Unknown;
        assert_eq!(
            evaluate_paper_risk(policy(), c),
            Err(ResearchRiskRefusal::UnverifiedFeed)
        );
        c.feed_quality = FeedQuality::Stale;
        assert_eq!(
            evaluate_paper_risk(policy(), c),
            Err(ResearchRiskRefusal::UnverifiedFeed)
        );
    }

    #[test]
    fn logical_time_must_be_consistent_and_fresh() {
        let mut c = candidate();
        c.quote_received_at_ms = 10_001;
        assert_eq!(
            evaluate_paper_risk(policy(), c),
            Err(ResearchRiskRefusal::ClockRegression)
        );
        c.quote_received_at_ms = 5_999;
        assert_eq!(
            evaluate_paper_risk(policy(), c),
            Err(ResearchRiskRefusal::StaleQuote)
        );
        c.quote_received_at_ms = 6_000;
        assert!(evaluate_paper_risk(policy(), c).is_ok());
    }

    #[test]
    fn exposure_and_daily_loss_limits_are_fail_closed() {
        let mut c = candidate();
        c.realized_daily_loss = UsdtMicros(20_000_000);
        assert_eq!(
            evaluate_paper_risk(policy(), c),
            Err(ResearchRiskRefusal::DailyLossLimit)
        );
        c.realized_daily_loss = UsdtMicros(0);
        c.order_notional = UsdtMicros(50_000_001);
        assert_eq!(
            evaluate_paper_risk(policy(), c),
            Err(ResearchRiskRefusal::OrderLimit)
        );
        c.order_notional = UsdtMicros(50_000_000);
        c.existing_exposure = UsdtMicros(100_000_001);
        assert_eq!(
            evaluate_paper_risk(policy(), c),
            Err(ResearchRiskRefusal::ExposureLimit)
        );
        c.existing_exposure = UsdtMicros(u128::MAX);
        assert_eq!(
            evaluate_paper_risk(policy(), c),
            Err(ResearchRiskRefusal::ExposureOverflow)
        );
    }

    #[test]
    fn missing_price_wide_spread_and_bad_configuration_are_refused() {
        let mut c = candidate();
        c.price_usdt_micros = UsdtMicros(0);
        assert_eq!(
            evaluate_paper_risk(policy(), c),
            Err(ResearchRiskRefusal::MissingPrice)
        );
        c.price_usdt_micros = UsdtMicros(100_000_000);
        c.spread_basis_points = 51;
        assert_eq!(
            evaluate_paper_risk(policy(), c),
            Err(ResearchRiskRefusal::ExcessiveSpread)
        );
        let mut p = policy();
        p.max_quote_age_ms = 0;
        assert_eq!(
            evaluate_paper_risk(p, candidate()),
            Err(ResearchRiskRefusal::InvalidLimits)
        );
    }

    #[test]
    fn identical_inputs_produce_identical_result_without_side_effects() {
        let input = candidate();
        let first = evaluate_paper_risk(policy(), input);
        let second = evaluate_paper_risk(policy(), input);
        assert_eq!(first, second);
        assert_eq!(
            first,
            Ok(PaperRiskAssessment {
                exposure_after: UsdtMicros(75_000_000),
            })
        );
    }
}
