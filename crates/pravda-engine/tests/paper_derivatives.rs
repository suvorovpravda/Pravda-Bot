use pravda_engine::paper_derivatives::{
    assess, settle, Candidate, Exit, Instrument, Limits, Mode, OptionRight, Refusal, Side,
};
use pravda_engine::research_risk::FeedQuality;

fn limits() -> Limits {
    Limits {
        paper_enabled: true,
        max_quote_age_ms: 1_000,
        max_order_micros: 200_000_000,
        max_gross_micros: 300_000_000,
        max_daily_loss_micros: 75_000_000,
        max_leverage_bps: 20_000,
        adverse_move_bps: 2_000,
        buffer_bps: 500,
    }
}

fn short() -> Instrument {
    Instrument::BorrowedShort {
        locate_verified: true,
        locate_until_ms: 20_000,
        initial_margin_bps: 15_000,
        maintenance_margin_bps: 8_000,
        cost_verified: true,
        carrying_cost_micros: 2_000_000,
    }
}

fn future(side: Side) -> Instrument {
    Instrument::CashSettledFuture {
        side,
        initial_margin_bps: 5_000,
        maintenance_margin_bps: 2_500,
        cost_verified: true,
        carrying_cost_micros: 1_000_000,
        expiry_ms: None,
    }
}

fn option(right: OptionRight) -> Instrument {
    Instrument::LongEuropeanOption {
        right,
        strike_micros: 11_000_000,
        total_premium_micros: 5_000_000,
        expiry_ms: 20_000,
    }
}

fn candidate(instrument: Instrument) -> Candidate {
    Candidate {
        mode: Mode::Paper,
        instrument,
        feed: FeedQuality::Verified,
        now_ms: 10_000,
        quote_ms: 9_500,
        price_micros: 10_000_000,
        quantity: 10,
        multiplier: 1,
        equity_micros: 300_000_000,
        free_collateral_micros: 250_000_000,
        existing_gross_micros: 50_000_000,
        realized_loss_micros: 0,
        unrealized_loss_micros: 0,
        entry_fees_micros: 1_000_000,
    }
}

fn exit(price_micros: u128, at_ms: u64) -> Exit {
    Exit {
        at_ms,
        price_micros,
        feed: FeedQuality::Verified,
        fees_micros: 1_000_000,
    }
}

#[test]
fn refuses_live_and_disabled_paper_mode() {
    let mut c = candidate(short());
    c.mode = Mode::Live;
    assert_eq!(assess(limits(), c), Err(Refusal::LiveDisabled));
    c.mode = Mode::Paper;
    let mut p = limits();
    p.paper_enabled = false;
    assert_eq!(assess(p, c), Err(Refusal::PaperDisabled));
}

#[test]
fn short_requires_locate_and_cost_verification() {
    let mut c = candidate(short());
    if let Instrument::BorrowedShort {
        ref mut locate_verified,
        ..
    } = c.instrument
    {
        *locate_verified = false;
    }
    assert_eq!(assess(limits(), c), Err(Refusal::BorrowUnavailable));
    c.instrument = short();
    if let Instrument::BorrowedShort {
        ref mut locate_until_ms,
        ..
    } = c.instrument
    {
        *locate_until_ms = 10_000;
    }
    assert_eq!(assess(limits(), c), Err(Refusal::BorrowExpired));
    c.instrument = short();
    if let Instrument::BorrowedShort {
        ref mut cost_verified,
        ..
    } = c.instrument
    {
        *cost_verified = false;
    }
    assert_eq!(assess(limits(), c), Err(Refusal::CostUnknown));
}

#[test]
fn gross_exposure_and_collateral_include_both_legs() {
    let result = assess(limits(), candidate(short())).unwrap();
    assert_eq!(result.notional_micros, 100_000_000);
    assert_eq!(result.gross_after_micros, 150_000_000);
    assert_eq!(result.stress_loss_micros, 20_000_000);
    assert_eq!(result.reserve_micros, 158_000_000);
    let mut c = candidate(short());
    c.existing_gross_micros = 250_000_001;
    assert_eq!(assess(limits(), c), Err(Refusal::GrossLimit));
    c.existing_gross_micros = 50_000_000;
    c.free_collateral_micros = 100_000_000;
    assert_eq!(assess(limits(), c), Err(Refusal::CollateralLimit));
    c.equity_micros = 70_000_000;
    c.free_collateral_micros = 60_000_000;
    assert_eq!(assess(limits(), c), Err(Refusal::LeverageLimit));
}

#[test]
fn losses_and_stale_quotes_are_rejected() {
    let mut c = candidate(future(Side::Long));
    c.realized_loss_micros = 75_000_000;
    assert_eq!(assess(limits(), c), Err(Refusal::DailyLossLimit));
    c.realized_loss_micros = 70_000_000;
    assert_eq!(assess(limits(), c), Err(Refusal::StressBudget));
    c.realized_loss_micros = 0;
    c.quote_ms = 8_999;
    assert_eq!(assess(limits(), c), Err(Refusal::StaleQuote));
    c.quote_ms = 10_001;
    assert_eq!(assess(limits(), c), Err(Refusal::ClockRegression));
    c.quote_ms = 9_500;
    c.feed = FeedQuality::Unknown;
    assert_eq!(assess(limits(), c), Err(Refusal::BadFeed));
}

#[test]
fn arithmetic_and_unknown_instruments_fail_closed() {
    let mut c = candidate(Instrument::Unsupported);
    assert_eq!(assess(limits(), c), Err(Refusal::UnsupportedInstrument));
    c.instrument = short();
    c.price_micros = u128::MAX;
    assert_eq!(assess(limits(), c), Err(Refusal::Overflow));
    c = candidate(future(Side::Long));
    if let Instrument::CashSettledFuture {
        ref mut maintenance_margin_bps,
        ..
    } = c.instrument
    {
        *maintenance_margin_bps = 5_000;
    }
    assert_eq!(assess(limits(), c), Err(Refusal::InvalidMargin));
}

#[test]
fn borrowed_short_pnl_deducts_borrow_and_all_fees() {
    let result = settle(limits(), candidate(short()), exit(8_000_000, 15_000)).unwrap();
    assert_eq!(result.gross_pnl_micros, 20_000_000);
    assert_eq!(result.net_pnl_micros, 16_000_000);
    assert_eq!(
        settle(limits(), candidate(short()), exit(8_000_000, 20_001)),
        Err(Refusal::BorrowExpired)
    );
}

#[test]
fn long_and_short_futures_settle_in_opposite_directions() {
    let long = settle(
        limits(),
        candidate(future(Side::Long)),
        exit(12_000_000, 15_000),
    )
    .unwrap();
    let short = settle(
        limits(),
        candidate(future(Side::Short)),
        exit(12_000_000, 15_000),
    )
    .unwrap();
    assert_eq!(long.gross_pnl_micros, 20_000_000);
    assert_eq!(long.net_pnl_micros, 17_000_000);
    assert_eq!(short.gross_pnl_micros, -20_000_000);
    assert_eq!(short.net_pnl_micros, -23_000_000);
}

#[test]
fn european_options_settle_at_expiry_only() {
    let call = settle(
        limits(),
        candidate(option(OptionRight::Call)),
        exit(12_000_000, 20_000),
    )
    .unwrap();
    let put = settle(
        limits(),
        candidate(option(OptionRight::Put)),
        exit(8_000_000, 20_000),
    )
    .unwrap();
    assert_eq!(call.gross_pnl_micros, 5_000_000);
    assert_eq!(call.net_pnl_micros, 3_000_000);
    assert_eq!(put.gross_pnl_micros, 25_000_000);
    assert_eq!(put.net_pnl_micros, 23_000_000);
    assert_eq!(
        settle(
            limits(),
            candidate(option(OptionRight::Call)),
            exit(12_000_000, 19_999)
        ),
        Err(Refusal::BadExit)
    );
}

#[test]
fn future_expiry_and_exit_quality_are_validated() {
    let mut c = candidate(future(Side::Long));
    c.instrument = Instrument::CashSettledFuture {
        side: Side::Long,
        initial_margin_bps: 5_000,
        maintenance_margin_bps: 2_500,
        cost_verified: true,
        carrying_cost_micros: 1_000_000,
        expiry_ms: Some(12_000),
    };
    assert_eq!(
        settle(limits(), c, exit(12_000_000, 15_000)),
        Err(Refusal::Expired)
    );
    let mut e = exit(12_000_000, 11_000);
    e.feed = FeedQuality::Stale;
    assert_eq!(settle(limits(), c, e), Err(Refusal::BadFeed));
}

#[test]
fn identical_inputs_have_identical_paper_outputs() {
    let c = candidate(short());
    assert_eq!(assess(limits(), c), assess(limits(), c));
    let e = exit(8_000_000, 15_000);
    assert_eq!(settle(limits(), c, e), settle(limits(), c, e));
}
