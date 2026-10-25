import math
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Mapping, Sequence


@dataclass(frozen=True)
class BookSnapshot:
    exchange: str
    bids: Sequence[Sequence[float]]
    asks: Sequence[Sequence[float]]
    received_at: datetime


@dataclass(frozen=True)
class MarketRules:
    min_amount: float = 0.0
    min_cost: float = 0.0
    amount_step: float = 0.0
    max_amount: float | None = None
    max_cost: float | None = None
    amount_significant_digits: int | None = None


@dataclass(frozen=True)
class OrderFill:
    exchange: str
    side: str
    amount: float
    average_price: float
    quote_value: float
    fee: float
    slippage_percent: float


@dataclass(frozen=True)
class ExecutionEstimate:
    status: str
    reason: str
    buy: OrderFill | None = None
    sell: OrderFill | None = None
    gross_pnl: float = 0.0
    fees: float = 0.0
    net_pnl: float = 0.0
    inventory_verified: bool = False


def _positive(value: object, name: str) -> float:
    try:
        parsed = float(value)
    except (TypeError, ValueError, OverflowError) as error:
        raise ValueError(f"{name} must be finite and positive") from error
    if not math.isfinite(parsed) or parsed <= 0:
        raise ValueError(f"{name} must be finite and positive")
    return parsed


def _validated_levels(levels: Sequence[Sequence[float]], name: str) -> list[tuple[float, float]]:
    result = []
    for level in levels:
        if len(level) < 2:
            raise ValueError(f"{name} contains an invalid level")
        price = _positive(level[0], f"{name} price")
        amount = _positive(level[1], f"{name} amount")
        result.append((price, amount))
    if not result:
        raise ValueError(f"{name} is empty")
    return result


def _round_amount_down(amount: float, step: float) -> float:
    if step <= 0:
        return amount
    return math.floor((amount + step * 1e-10) / step) * step


def _amount_step(amount: float, rules: MarketRules) -> float:
    step = rules.amount_step
    digits = rules.amount_significant_digits
    if digits is not None:
        if digits <= 0:
            raise ValueError("amount significant digits must be positive")
        significant_step = 10 ** (math.floor(math.log10(amount)) - digits + 1)
        step = max(step, significant_step)
    return step


def _walk_book(levels: list[tuple[float, float]], amount: float) -> tuple[float, float]:
    remaining = amount
    quote_value = 0.0
    filled = 0.0
    for price, available in levels:
        take = min(remaining, available)
        filled += take
        quote_value += take * price
        remaining -= take
        if remaining <= max(1e-12, amount * 1e-12):
            break
    return filled, quote_value


def _fill(
    exchange: str,
    side: str,
    levels: list[tuple[float, float]],
    amount: float,
    fee_rate: float,
) -> OrderFill:
    filled, quote_value = _walk_book(levels, amount)
    if not math.isclose(filled, amount, rel_tol=1e-9, abs_tol=1e-12):
        raise ValueError("insufficient order-book depth")
    average_price = quote_value / amount
    top_price = levels[0][0]
    slippage_percent = (
        (average_price - top_price) / top_price * 100
        if side == "buy"
        else (top_price - average_price) / top_price * 100
    )
    return OrderFill(
        exchange=exchange,
        side=side,
        amount=amount,
        average_price=average_price,
        quote_value=quote_value,
        fee=quote_value * fee_rate,
        slippage_percent=max(0.0, slippage_percent),
    )


def estimate_cross_exchange_execution(
    books: Mapping[str, BookSnapshot],
    market_rules: Mapping[str, MarketRules],
    *,
    target_notional: float,
    taker_fee_percent: float,
    max_slippage_percent: float,
    max_book_age_seconds: float,
    now: datetime | None = None,
) -> ExecutionEstimate:
    """Estimate a spot buy/sell using visible depth; account inventory is not checked."""
    target_notional = _positive(target_notional, "target_notional")
    for name, value in (
        ("taker_fee_percent", taker_fee_percent),
        ("max_slippage_percent", max_slippage_percent),
        ("max_book_age_seconds", max_book_age_seconds),
    ):
        if not math.isfinite(value) or value < 0:
            raise ValueError(f"{name} must be finite and non-negative")
    if len(books) != 2 or len(market_rules) != 2 or set(books) != set(market_rules):
        raise ValueError("exactly two matching exchange books and market rules are required")

    current_time = now or datetime.now(timezone.utc)
    if current_time.tzinfo is None:
        raise ValueError("current time must include a timezone")
    current_time = current_time.astimezone(timezone.utc)
    normalized = {}
    for exchange, book in books.items():
        if book.exchange != exchange:
            raise ValueError("book exchange does not match its mapping key")
        if book.received_at.tzinfo is None:
            raise ValueError("book timestamps must include a timezone")
        age = (current_time - book.received_at.astimezone(timezone.utc)).total_seconds()
        if age < 0 or age > max_book_age_seconds:
            return ExecutionEstimate("rejected", f"{exchange} order book is stale")
        bids = sorted(
            _validated_levels(book.bids, f"{exchange} bids"),
            key=lambda level: level[0],
            reverse=True,
        )
        asks = sorted(
            _validated_levels(book.asks, f"{exchange} asks"),
            key=lambda level: level[0],
        )
        if bids[0][0] >= asks[0][0]:
            raise ValueError(f"{exchange} order book is crossed")
        normalized[exchange] = (bids, asks)
        rules = market_rules[exchange]
        for name, value in (
            ("minimum amount", rules.min_amount),
            ("minimum notional", rules.min_cost),
            ("amount step", rules.amount_step),
            ("maximum amount", rules.max_amount),
            ("maximum notional", rules.max_cost),
        ):
            if value is not None and (not math.isfinite(value) or value < 0):
                raise ValueError(f"{exchange} {name} must be finite and non-negative")
        if (
            rules.amount_significant_digits is not None
            and rules.amount_significant_digits <= 0
        ):
            raise ValueError(f"{exchange} amount significant digits must be positive")

    buy_exchange = min(books, key=lambda exchange: normalized[exchange][1][0][0])
    sell_exchange = max(books, key=lambda exchange: normalized[exchange][0][0][0])
    if buy_exchange == sell_exchange:
        return ExecutionEstimate("rejected", "no cross-exchange top-of-book opportunity")

    buy_asks = normalized[buy_exchange][1]
    sell_bids = normalized[sell_exchange][0]
    requested_amount = target_notional / buy_asks[0][0]
    buy_rule = market_rules[buy_exchange]
    sell_rule = market_rules[sell_exchange]
    amount_step = max(
        _amount_step(requested_amount, buy_rule),
        _amount_step(requested_amount, sell_rule),
    )
    requested_amount = _round_amount_down(requested_amount, amount_step)
    available_amount = min(
        sum(amount for _, amount in buy_asks),
        sum(amount for _, amount in sell_bids),
    )
    amount = _round_amount_down(min(requested_amount, available_amount), amount_step)
    if amount <= 0:
        return ExecutionEstimate("rejected", "insufficient order-book depth")

    try:
        buy_fill = _fill(buy_exchange, "buy", buy_asks, amount, taker_fee_percent / 100)
        sell_fill = _fill(sell_exchange, "sell", sell_bids, amount, taker_fee_percent / 100)
    except ValueError:
        return ExecutionEstimate("rejected", "insufficient order-book depth")

    for exchange, fill in ((buy_exchange, buy_fill), (sell_exchange, sell_fill)):
        rules = market_rules[exchange]
        if fill.amount < rules.min_amount:
            return ExecutionEstimate("rejected", f"{exchange} minimum amount not met")
        if fill.quote_value < rules.min_cost:
            return ExecutionEstimate("rejected", f"{exchange} minimum notional not met")
        if rules.max_amount is not None and fill.amount > rules.max_amount:
            return ExecutionEstimate("rejected", f"{exchange} maximum amount exceeded")
        if rules.max_cost is not None and fill.quote_value > rules.max_cost:
            return ExecutionEstimate("rejected", f"{exchange} maximum notional exceeded")

    fees = buy_fill.fee + sell_fill.fee
    gross_pnl = sell_fill.quote_value - buy_fill.quote_value
    net_pnl = gross_pnl - fees
    if buy_fill.slippage_percent + sell_fill.slippage_percent > max_slippage_percent:
        return ExecutionEstimate(
            "rejected",
            "slippage limit exceeded",
            buy_fill,
            sell_fill,
            gross_pnl,
            fees,
            net_pnl,
        )
    if net_pnl <= 0:
        return ExecutionEstimate(
            "rejected",
            "estimated net profit is not positive",
            buy_fill,
            sell_fill,
            gross_pnl,
            fees,
            net_pnl,
        )
    partial = amount < requested_amount
    return ExecutionEstimate(
        "partial_estimate" if partial else "estimate_only",
        (
            "partial book fill; account inventory is unverified"
            if partial
            else "book depth and configured market rules pass; account inventory is unverified"
        ),
        buy_fill,
        sell_fill,
        gross_pnl,
        fees,
        net_pnl,
    )


def fetch_public_execution_estimate(
    symbol: str,
    *,
    target_notional: float,
    taker_fee_percent: float,
    max_slippage_percent: float,
    max_book_age_seconds: float,
) -> ExecutionEstimate:
    import ccxt

    exchanges = {
        "Kraken": ccxt.kraken({"enableRateLimit": True}),
        "Binance": ccxt.binance({"enableRateLimit": True}),
    }
    books = {}
    rules = {}
    try:
        for name, exchange in exchanges.items():
            markets = exchange.load_markets()
            market = markets.get(symbol)
            if market is None:
                raise ValueError(f"{name} does not provide the {symbol} market")
            raw_book = exchange.fetch_order_book(symbol, limit=50)
            books[name] = BookSnapshot(
                exchange=name,
                bids=raw_book.get("bids", []),
                asks=raw_book.get("asks", []),
                received_at=datetime.now(timezone.utc),
            )
            limits = market.get("limits", {})
            precision = market.get("precision", {}).get("amount")
            amount_step = 0.0
            amount_significant_digits = None
            if precision is not None:
                if exchange.precisionMode == ccxt.TICK_SIZE:
                    amount_step = float(precision)
                elif exchange.precisionMode == ccxt.DECIMAL_PLACES:
                    amount_step = 10 ** -int(precision)
                elif exchange.precisionMode == ccxt.SIGNIFICANT_DIGITS:
                    amount_significant_digits = int(precision)
            rules[name] = MarketRules(
                min_amount=float((limits.get("amount") or {}).get("min") or 0),
                min_cost=float((limits.get("cost") or {}).get("min") or 0),
                amount_step=amount_step,
                max_amount=(
                    float((limits.get("amount") or {}).get("max"))
                    if (limits.get("amount") or {}).get("max") is not None
                    else None
                ),
                max_cost=(
                    float((limits.get("cost") or {}).get("max"))
                    if (limits.get("cost") or {}).get("max") is not None
                    else None
                ),
                amount_significant_digits=amount_significant_digits,
            )
    finally:
        for exchange in exchanges.values():
            exchange.close()

    return estimate_cross_exchange_execution(
        books,
        rules,
        target_notional=target_notional,
        taker_fee_percent=taker_fee_percent,
        max_slippage_percent=max_slippage_percent,
        max_book_age_seconds=max_book_age_seconds,
    )
