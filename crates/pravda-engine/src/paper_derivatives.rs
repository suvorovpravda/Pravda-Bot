//! Deterministic paper-only risk calculations. No exchange API, credentials,
//! persistence, order execution, background process, or clock dependency.
//! All amounts are micro-USDT and all quantities are whole underlying units.
//! A stress scenario is NOT a bound on real-world losses or liquidation risk.

use crate::research_risk::FeedQuality;

const BPS: u128 = 10_000;

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
pub enum OptionRight {
    Call,
    Put,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instrument {
    BorrowedShort {
        locate_verified: bool,
        locate_until_ms: u64,
        initial_margin_bps: u32,
        maintenance_margin_bps: u32,
        cost_verified: bool,
        carrying_cost_micros: u128,
    },
    CashSettledFuture {
        side: Side,
        initial_margin_bps: u32,
        maintenance_margin_bps: u32,
        cost_verified: bool,
        carrying_cost_micros: u128,
        expiry_ms: Option<u64>,
    },
    LongEuropeanOption {
        right: OptionRight,
        strike_micros: u128,
        total_premium_micros: u128,
        expiry_ms: u64,
    },
    Unsupported,
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub paper_enabled: bool,
    pub max_quote_age_ms: u64,
    pub max_order_micros: u128,
    pub max_gross_micros: u128,
    pub max_daily_loss_micros: u128,
    pub max_leverage_bps: u32,
    pub adverse_move_bps: u32,
    pub buffer_bps: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Candidate {
    pub mode: Mode,
    pub instrument: Instrument,
    pub feed: FeedQuality,
    pub now_ms: u64,
    pub quote_ms: u64,
    pub price_micros: u128,
    pub quantity: u64,
    pub multiplier: u64,
    pub equity_micros: u128,
    pub free_collateral_micros: u128,
    pub existing_gross_micros: u128,
    pub realized_loss_micros: u128,
    pub unrealized_loss_micros: u128,
    pub entry_fees_micros: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Assessment {
    pub notional_micros: u128,
    pub gross_after_micros: u128,
    pub stress_loss_micros: u128,
    pub reserve_micros: u128,
}

#[derive(Debug, Clone, Copy)]
pub struct Exit {
    pub at_ms: u64,
    pub price_micros: u128,
    pub feed: FeedQuality,
    pub fees_micros: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settlement {
    pub gross_pnl_micros: i128,
    pub net_pnl_micros: i128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    LiveDisabled,
    PaperDisabled,
    InvalidPolicy,
    BadFeed,
    BadQuote,
    ClockRegression,
    StaleQuote,
    BadSnapshot,
    UnsupportedInstrument,
    BorrowUnavailable,
    BorrowExpired,
    CostUnknown,
    InvalidMargin,
    Expired,
    OrderLimit,
    GrossLimit,
    LeverageLimit,
    DailyLossLimit,
    StressBudget,
    CollateralLimit,
    BadExit,
    Overflow,
}

fn add(a: u128, b: u128) -> Result<u128, Refusal> {
    a.checked_add(b).ok_or(Refusal::Overflow)
}

fn mul(a: u128, b: u128) -> Result<u128, Refusal> {
    a.checked_mul(b).ok_or(Refusal::Overflow)
}

fn ceil_bps(value: u128, bps: u32) -> Result<u128, Refusal> {
    let result = mul(value, u128::from(bps))?;
    add(result / BPS, u128::from(result % BPS != 0))
}

fn instrument_terms(i: Instrument, now: u64) -> Result<(u32, u128), Refusal> {
    match i {
        Instrument::BorrowedShort {
            locate_verified,
            locate_until_ms,
            initial_margin_bps,
            maintenance_margin_bps,
            cost_verified,
            carrying_cost_micros,
        } => {
            if !locate_verified {
                return Err(Refusal::BorrowUnavailable);
            }
            if locate_until_ms <= now {
                return Err(Refusal::BorrowExpired);
            }
            if !cost_verified {
                return Err(Refusal::CostUnknown);
            }
            if maintenance_margin_bps == 0 || initial_margin_bps <= maintenance_margin_bps {
                return Err(Refusal::InvalidMargin);
            }
            Ok((initial_margin_bps, carrying_cost_micros))
        }
        Instrument::CashSettledFuture {
            initial_margin_bps,
            maintenance_margin_bps,
            cost_verified,
            carrying_cost_micros,
            expiry_ms,
            ..
        } => {
            if expiry_ms.is_some_and(|x| x <= now) {
                return Err(Refusal::Expired);
            }
            if !cost_verified {
                return Err(Refusal::CostUnknown);
            }
            if maintenance_margin_bps == 0 || initial_margin_bps <= maintenance_margin_bps {
                return Err(Refusal::InvalidMargin);
            }
            Ok((initial_margin_bps, carrying_cost_micros))
        }
        Instrument::LongEuropeanOption {
            strike_micros,
            total_premium_micros,
            expiry_ms,
            ..
        } => {
            if expiry_ms <= now {
                return Err(Refusal::Expired);
            }
            if strike_micros == 0 || total_premium_micros == 0 {
                return Err(Refusal::UnsupportedInstrument);
            }
            Ok((0, 0))
        }
        Instrument::Unsupported => Err(Refusal::UnsupportedInstrument),
    }
}

/// A positive result is only a hypothetical paper assessment.
/// Gross exposure counts opposite positions without netting them away.
pub fn assess(limits: Limits, c: Candidate) -> Result<Assessment, Refusal> {
    if c.mode != Mode::Paper {
        return Err(Refusal::LiveDisabled);
    }
    if !limits.paper_enabled {
        return Err(Refusal::PaperDisabled);
    }
    if limits.max_quote_age_ms == 0
        || limits.max_order_micros == 0
        || limits.max_gross_micros < limits.max_order_micros
        || limits.max_daily_loss_micros == 0
        || limits.max_leverage_bps == 0
        || limits.adverse_move_bps == 0
    {
        return Err(Refusal::InvalidPolicy);
    }
    if c.feed != FeedQuality::Verified {
        return Err(Refusal::BadFeed);
    }
    if c.price_micros == 0 {
        return Err(Refusal::BadQuote);
    }
    let Some(age) = c.now_ms.checked_sub(c.quote_ms) else {
        return Err(Refusal::ClockRegression);
    };
    if age > limits.max_quote_age_ms {
        return Err(Refusal::StaleQuote);
    }
    if c.quantity == 0
        || c.multiplier == 0
        || c.equity_micros == 0
        || c.free_collateral_micros > c.equity_micros
    {
        return Err(Refusal::BadSnapshot);
    }
    let (margin_bps, carry) = instrument_terms(c.instrument, c.now_ms)?;
    let units = mul(u128::from(c.quantity), u128::from(c.multiplier))?;
    let notional = mul(units, c.price_micros)?;
    if notional > limits.max_order_micros {
        return Err(Refusal::OrderLimit);
    }
    let gross = add(c.existing_gross_micros, notional)?;
    if gross > limits.max_gross_micros {
        return Err(Refusal::GrossLimit);
    }
    if mul(gross, BPS)? > mul(c.equity_micros, u128::from(limits.max_leverage_bps))? {
        return Err(Refusal::LeverageLimit);
    }
    let previous_loss = add(c.realized_loss_micros, c.unrealized_loss_micros)?;
    if previous_loss >= limits.max_daily_loss_micros {
        return Err(Refusal::DailyLossLimit);
    }
    let (initial_margin, stress) = match c.instrument {
        Instrument::LongEuropeanOption {
            total_premium_micros,
            ..
        } => (total_premium_micros, total_premium_micros),
        _ => (
            ceil_bps(notional, margin_bps)?,
            ceil_bps(notional, limits.adverse_move_bps)?,
        ),
    };
    let entry_costs = add(c.entry_fees_micros, carry)?;
    if add(add(previous_loss, stress)?, entry_costs)? > limits.max_daily_loss_micros {
        return Err(Refusal::StressBudget);
    }
    let reserve = add(
        add(initial_margin.max(stress), ceil_bps(notional, limits.buffer_bps)?)?,
        entry_costs,
    )?;
    if reserve > c.free_collateral_micros {
        return Err(Refusal::CollateralLimit);
    }
    Ok(Assessment {
        notional_micros: notional,
        gross_after_micros: gross,
        stress_loss_micros: stress,
        reserve_micros: reserve,
    })
}

fn signed(amount: u128) -> Result<i128, Refusal> {
    i128::try_from(amount).map_err(|_| Refusal::Overflow)
}

/// Pure hypothetical PnL, not a simulated fill, margin engine or live price.
pub fn settle(limits: Limits, c: Candidate, e: Exit) -> Result<Settlement, Refusal> {
    assess(limits, c)?;
    if e.feed != FeedQuality::Verified {
        return Err(Refusal::BadFeed);
    }
    if e.at_ms < c.now_ms || e.price_micros == 0 {
        return Err(Refusal::BadExit);
    }
    let units = signed(mul(u128::from(c.quantity), u128::from(c.multiplier))?)?;
    let carrying = match c.instrument {
        Instrument::BorrowedShort {
            locate_until_ms,
            carrying_cost_micros,
            ..
        } => {
            if e.at_ms > locate_until_ms {
                return Err(Refusal::BorrowExpired);
            }
            carrying_cost_micros
        }
        Instrument::CashSettledFuture {
            expiry_ms,
            carrying_cost_micros,
            ..
        } => {
            if expiry_ms.is_some_and(|x| e.at_ms > x) {
                return Err(Refusal::Expired);
            }
            carrying_cost_micros
        }
        Instrument::LongEuropeanOption { expiry_ms, .. } => {
            if e.at_ms != expiry_ms {
                return Err(Refusal::BadExit);
            }
            0
        }
        Instrument::Unsupported => return Err(Refusal::UnsupportedInstrument),
    };
    let gross = match c.instrument {
        Instrument::BorrowedShort { .. }
        | Instrument::CashSettledFuture {
            side: Side::Short,
            ..
        } => signed(c.price_micros)?
            .checked_sub(signed(e.price_micros)?)
            .and_then(|x| x.checked_mul(units))
            .ok_or(Refusal::Overflow)?,
        Instrument::CashSettledFuture {
            side: Side::Long,
            ..
        } => signed(e.price_micros)?
            .checked_sub(signed(c.price_micros)?)
            .and_then(|x| x.checked_mul(units))
            .ok_or(Refusal::Overflow)?,
        Instrument::LongEuropeanOption {
            right,
            strike_micros,
            total_premium_micros,
            ..
        } => {
            let intrinsic = match right {
                OptionRight::Call => e.price_micros.saturating_sub(strike_micros),
                OptionRight::Put => strike_micros.saturating_sub(e.price_micros),
            };
            signed(mul(intrinsic, u128::try_from(units).map_err(|_| Refusal::Overflow)?)?)?
                .checked_sub(signed(total_premium_micros)?)
                .ok_or(Refusal::Overflow)?
        }
        Instrument::Unsupported => return Err(Refusal::UnsupportedInstrument),
    };
    let fees = signed(add(add(c.entry_fees_micros, e.fees_micros)?, carrying)?)?;
    let net = gross.checked_sub(fees).ok_or(Refusal::Overflow)?;
    Ok(Settlement {
        gross_pnl_micros: gross,
        net_pnl_micros: net,
    })
}
