import math
import os
from dataclasses import dataclass
from typing import Mapping
from urllib.parse import urlsplit


@dataclass(frozen=True)
class BotConfig:
    lm_url: str
    trading_mode: str
    model_name: str
    min_spread_threshold: float
    max_allowed_spread_percent: float
    spread_log_interval_seconds: float
    price_alert_cooldown_seconds: float
    ntfy_global_cooldown_seconds: float
    dashboard_interval_seconds: float
    alert_confirmation_seconds: float
    alert_rearm_hysteresis_percent: float
    market_data_stale_seconds: float
    market_data_max_skew_seconds: float
    paper_mode: str
    paper_starting_balance_usdt: float
    paper_position_percent: float
    paper_fixed_position_usdt: float
    paper_exit_spread_percent: float
    paper_max_hold_seconds: float
    ntfy_topic_url: str


def _float_setting(environment, name, default, *, allow_zero):
    try:
        value = float(environment.get(name, str(default)))
    except (TypeError, ValueError) as error:
        raise ValueError(f"{name} must be a finite non-negative number") from error
    if not math.isfinite(value) or value < 0 or (not allow_zero and value == 0):
        qualifier = "non-negative" if allow_zero else "greater than zero"
        raise ValueError(f"{name} must be a finite number {qualifier}")
    return value


def _validate_url(name, value, *, https_only=False):
    try:
        parsed = urlsplit(value)
        parsed.port
    except ValueError as error:
        raise ValueError(f"{name} must be a valid absolute URL") from error
    allowed_schemes = {"https"} if https_only else {"http", "https"}
    if (
        parsed.scheme not in allowed_schemes
        or not parsed.hostname
        or parsed.username is not None
        or parsed.password is not None
    ):
        schemes = "https" if https_only else "http or https"
        raise ValueError(f"{name} must be an absolute {schemes} URL without embedded credentials")


def load_config(environment: Mapping[str, str] | None = None) -> BotConfig:
    values = os.environ if environment is None else environment
    lm_url = values.get("LM_STUDIO_API_BASE", "http://192.168.1.146:1234/v1").strip()
    model_name = values.get("LM_MODEL_NAME", "qwen/qwen3.5-9b").strip()
    trading_mode = values.get("TRADING_MODE", "paper").strip()
    paper_mode = values.get("PAPER_MODE", "portfolio").strip().lower()
    ntfy_topic_url = values.get("NTFY_TOPIC_URL", "").strip()

    _validate_url("LM_STUDIO_API_BASE", lm_url)
    if not model_name:
        raise ValueError("LM_MODEL_NAME must not be empty")
    if not trading_mode:
        raise ValueError("TRADING_MODE must not be empty")
    if paper_mode not in {"fixed", "portfolio"}:
        raise ValueError("PAPER_MODE must be either 'fixed' or 'portfolio'")
    if ntfy_topic_url:
        _validate_url("NTFY_TOPIC_URL", ntfy_topic_url, https_only=True)

    paper_starting_balance = _float_setting(
        values, "PAPER_STARTING_BALANCE_USDT", 1000, allow_zero=False
    )
    paper_position_percent = _float_setting(
        values, "PAPER_POSITION_PERCENT", 10, allow_zero=False
    )
    if paper_position_percent > 100:
        raise ValueError("PAPER_POSITION_PERCENT must not exceed 100")

    return BotConfig(
        lm_url=lm_url.rstrip("/"),
        trading_mode=trading_mode,
        model_name=model_name,
        min_spread_threshold=_float_setting(
            values, "MIN_SPREAD_THRESHOLD", 0.05, allow_zero=True
        ),
        max_allowed_spread_percent=_float_setting(
            values, "MAX_ALLOWED_SPREAD_PERCENT", 5.0, allow_zero=False
        ),
        spread_log_interval_seconds=_float_setting(
            values, "SPREAD_LOG_INTERVAL_SECONDS", 2, allow_zero=False
        ),
        price_alert_cooldown_seconds=_float_setting(
            values, "PRICE_ALERT_COOLDOWN_SECONDS", 5, allow_zero=True
        ),
        ntfy_global_cooldown_seconds=_float_setting(
            values, "NTFY_GLOBAL_COOLDOWN_SECONDS", 10, allow_zero=False
        ),
        dashboard_interval_seconds=_float_setting(
            values, "DASHBOARD_INTERVAL_SECONDS", 30, allow_zero=False
        ),
        alert_confirmation_seconds=_float_setting(
            values, "ALERT_CONFIRMATION_SECONDS", 3, allow_zero=True
        ),
        alert_rearm_hysteresis_percent=_float_setting(
            values, "ALERT_REARM_HYSTERESIS_PERCENT", 0.05, allow_zero=True
        ),
        market_data_stale_seconds=_float_setting(
            values, "MARKET_DATA_STALE_SECONDS", 4, allow_zero=False
        ),
        market_data_max_skew_seconds=_float_setting(
            values, "MARKET_DATA_MAX_SKEW_SECONDS", 3, allow_zero=False
        ),
        paper_mode=paper_mode,
        paper_starting_balance_usdt=paper_starting_balance,
        paper_position_percent=paper_position_percent,
        paper_fixed_position_usdt=_float_setting(
            values, "PAPER_FIXED_POSITION_USDT", 100, allow_zero=False
        ),
        paper_exit_spread_percent=_float_setting(
            values, "PAPER_EXIT_SPREAD_PERCENT", 0.05, allow_zero=True
        ),
        paper_max_hold_seconds=_float_setting(
            values, "PAPER_MAX_HOLD_SECONDS", 3600, allow_zero=False
        ),
        ntfy_topic_url=ntfy_topic_url,
    )
