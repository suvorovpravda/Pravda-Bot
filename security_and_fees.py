import logging
import math


logger = logging.getLogger("PravdaSecurity")
DEFAULT_ESTIMATED_FEES_PERCENT = 0.36
DEFAULT_ESTIMATED_FEES_RATE = DEFAULT_ESTIMATED_FEES_PERCENT / 100
DEFAULT_MAX_ALLOWED_SPREAD_PERCENT = 5.0


def request_error_summary(error: BaseException) -> str:
    """Describe a network error without exposing its URL or request details."""
    exception_name = type(error).__name__
    response = getattr(error, "response", None)
    status_code = getattr(response, "status_code", None)
    if isinstance(status_code, int) and not isinstance(status_code, bool):
        return f"{exception_name} (HTTP {status_code})"
    return exception_name


def parse_positive_price(raw_price: object) -> float:
    """Parse a finite positive market price received from an exchange stream."""
    try:
        price = float(raw_price)
    except (TypeError, ValueError, OverflowError) as error:
        raise ValueError("price must be finite and positive") from error
    if not math.isfinite(price) or price <= 0:
        raise ValueError("price must be finite and positive")
    return price


def calculate_net_spread(
    gross_spread_percent: float,
    estimated_fees_percent: float = DEFAULT_ESTIMATED_FEES_PERCENT,
) -> float:
    """Subtract the configured percentage-point fee estimate from the gross spread."""
    if not math.isfinite(gross_spread_percent) or gross_spread_percent < 0:
        raise ValueError("gross_spread_percent must be finite and non-negative")
    if not math.isfinite(estimated_fees_percent) or estimated_fees_percent < 0:
        raise ValueError("estimated_fees_percent must be finite and non-negative")
    return gross_spread_percent - estimated_fees_percent


def calculate_net_spread_safe(
    price_1: float,
    price_2: float,
    fee_rate: float = DEFAULT_ESTIMATED_FEES_RATE,
) -> float:
    """Return net spread as a ratio (0.01 means 1%), rejecting invalid inputs."""
    price_1 = parse_positive_price(price_1)
    price_2 = parse_positive_price(price_2)
    try:
        fee_rate = float(fee_rate)
    except (TypeError, ValueError, OverflowError) as error:
        raise ValueError("fee_rate must be finite and non-negative") from error
    if not math.isfinite(fee_rate) or fee_rate < 0:
        raise ValueError("fee_rate must be finite and non-negative")

    gross_spread_rate = abs(price_1 - price_2) / min(price_1, price_2)
    if not math.isfinite(gross_spread_rate):
        raise ValueError("gross spread rate must be finite")
    return gross_spread_rate - fee_rate


def check_security_guardrails(
    spread_percent: float,
    max_allowed_spread_percent: float = DEFAULT_MAX_ALLOWED_SPREAD_PERCENT,
) -> bool:
    """Reject invalid or implausibly large gross spreads before simulation or alerts."""
    if not math.isfinite(max_allowed_spread_percent) or max_allowed_spread_percent <= 0:
        raise ValueError("max_allowed_spread_percent must be finite and greater than zero")
    if not math.isfinite(spread_percent) or spread_percent <= 0:
        return False
    if spread_percent > max_allowed_spread_percent:
        logger.warning(
            "Anomalous gross spread rejected: %.4f%% exceeds configured maximum %.4f%%",
            spread_percent,
            max_allowed_spread_percent,
        )
        return False
    return True
