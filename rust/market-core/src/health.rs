//! Market freshness checks based on monotonic timestamps and quote generations.
use std::time::{Duration, Instant};

const FUTURE_TOLERANCE: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthError {
    InvalidThreshold,
    InvalidTimestamp,
    FutureTimestamp,
    TimestampOutOfRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Waiting,
    Stale,
    Desynchronized,
    Ok,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketDataHealth {
    pub status: HealthStatus,
    pub kraken_age: Option<Duration>,
    pub binance_age: Option<Duration>,
    pub exchange_skew: Option<Duration>,
}

/// Distinct variants prevent accidental interpretation of milliseconds as seconds.
#[derive(Debug, Clone, Copy)]
pub enum EventTimestamp<'a> {
    UnixSeconds(f64),
    UnixMilliseconds(f64),
    Rfc3339(&'a str),
}

/// Convert an exchange timestamp using wall and monotonic clocks sampled together.
/// Clock skew up to five seconds in the future is preserved for the skew check.
pub fn event_to_monotonic(
    timestamp: EventTimestamp<'_>,
    received_epoch_seconds: f64,
    received_monotonic: Instant,
) -> Result<Instant, HealthError> {
    let epoch: f64 = match timestamp {
        EventTimestamp::UnixSeconds(seconds) => seconds,
        EventTimestamp::UnixMilliseconds(milliseconds) => milliseconds / 1000.0_f64,
        EventTimestamp::Rfc3339(text) => {
            let parsed: chrono::DateTime<chrono::FixedOffset> =
                chrono::DateTime::parse_from_rfc3339(text)
                    .map_err(|_| HealthError::InvalidTimestamp)?;
            parsed.timestamp() as f64 + f64::from(parsed.timestamp_subsec_nanos()) / 1e9_f64
        }
    };
    if !epoch.is_finite() || !received_epoch_seconds.is_finite() {
        return Err(HealthError::InvalidTimestamp);
    }
    let age: f64 = received_epoch_seconds - epoch;
    if !age.is_finite() {
        return Err(HealthError::TimestampOutOfRange);
    }
    if age < -FUTURE_TOLERANCE.as_secs_f64() {
        return Err(HealthError::FutureTimestamp);
    }
    let duration: Duration =
        Duration::try_from_secs_f64(age.abs()).map_err(|_| HealthError::TimestampOutOfRange)?;
    if age >= 0.0_f64 {
        received_monotonic.checked_sub(duration)
    } else {
        received_monotonic.checked_add(duration)
    }
    .ok_or(HealthError::TimestampOutOfRange)
}

/// Assess each pair before use. Missing quotes never become healthy.
pub fn assess_market_data(
    kraken: Option<Instant>,
    binance: Option<Instant>,
    now: Instant,
    stale_after: Duration,
    max_skew: Duration,
) -> Result<MarketDataHealth, HealthError> {
    if stale_after.is_zero() {
        return Err(HealthError::InvalidThreshold);
    }
    for timestamp in [kraken, binance].into_iter().flatten() {
        if timestamp > now && timestamp.duration_since(now) > FUTURE_TOLERANCE {
            return Err(HealthError::FutureTimestamp);
        }
    }
    let (Some(kraken), Some(binance)) = (kraken, binance) else {
        return Ok(MarketDataHealth {
            status: HealthStatus::Waiting,
            kraken_age: None,
            binance_age: None,
            exchange_skew: None,
        });
    };
    let kraken_age: Duration = now.checked_duration_since(kraken).unwrap_or(Duration::ZERO);
    let binance_age: Duration = now
        .checked_duration_since(binance)
        .unwrap_or(Duration::ZERO);
    let skew: Duration = if kraken >= binance {
        kraken.duration_since(binance)
    } else {
        binance.duration_since(kraken)
    };
    let status: HealthStatus = if kraken_age > stale_after || binance_age > stale_after {
        HealthStatus::Stale
    } else if skew > max_skew {
        HealthStatus::Desynchronized
    } else {
        HealthStatus::Ok
    };
    Ok(MarketDataHealth {
        status,
        kraken_age: Some(kraken_age),
        binance_age: Some(binance_age),
        exchange_skew: Some(skew),
    })
}

/// Return None if either quote changed after the snapshot was captured.
pub fn assess_snapshot(
    snapshot_generation: [Option<u64>; 2],
    current_generation: [Option<u64>; 2],
    kraken: Option<Instant>,
    binance: Option<Instant>,
    now: Instant,
    stale_after: Duration,
    max_skew: Duration,
) -> Result<Option<MarketDataHealth>, HealthError> {
    if snapshot_generation.iter().any(Option::is_none)
        || current_generation.iter().any(Option::is_none)
        || snapshot_generation != current_generation
    {
        return Ok(None);
    }
    assess_market_data(kraken, binance, now, stale_after, max_skew).map(Some)
}

#[cfg(test)]
mod tests {
    use super::{
        assess_market_data, assess_snapshot, event_to_monotonic, EventTimestamp, HealthError,
        HealthStatus,
    };
    use std::time::{Duration, Instant};

    #[test]
    fn waits_then_distinguishes_fresh_stale_and_desynchronized() {
        let now: Instant = Instant::now();
        let stale_after: Duration = Duration::from_secs(4);
        let skew: Duration = Duration::from_secs(3);
        assert_eq!(
            assess_market_data(None, Some(now), now, stale_after, skew)
                .expect("valid")
                .status,
            HealthStatus::Waiting
        );
        assert_eq!(
            assess_market_data(
                Some(now - Duration::from_secs(4)),
                Some(now - Duration::from_secs(1)),
                now,
                stale_after,
                skew
            )
            .expect("valid")
            .status,
            HealthStatus::Ok
        );
        assert_eq!(
            assess_market_data(
                Some(now - Duration::from_millis(4001)),
                Some(now),
                now,
                stale_after,
                skew
            )
            .expect("valid")
            .status,
            HealthStatus::Stale
        );
        assert_eq!(
            assess_market_data(
                Some(now - Duration::from_millis(3001)),
                Some(now),
                now,
                stale_after,
                skew
            )
            .expect("valid")
            .status,
            HealthStatus::Desynchronized
        );
        assert_eq!(
            assess_market_data(
                Some(now - Duration::from_secs(6)),
                Some(now),
                now,
                stale_after,
                skew
            )
            .expect("valid")
            .status,
            HealthStatus::Stale
        );
    }

    #[test]
    fn rejects_invalid_threshold_and_far_future_even_when_quote_missing() {
        let now: Instant = Instant::now();
        assert_eq!(
            assess_market_data(None, None, now, Duration::ZERO, Duration::ZERO),
            Err(HealthError::InvalidThreshold)
        );
        assert_eq!(
            assess_market_data(
                Some(now + Duration::from_secs(6)),
                None,
                now,
                Duration::from_secs(4),
                Duration::from_secs(3)
            ),
            Err(HealthError::FutureTimestamp)
        );
        assert_eq!(
            assess_market_data(
                Some(now + Duration::from_secs(5)),
                Some(now),
                now,
                Duration::from_secs(4),
                Duration::from_secs(5)
            )
            .expect("valid")
            .status,
            HealthStatus::Ok
        );
    }

    #[test]
    fn invalidated_snapshot_never_returns_a_health_assessment() {
        let now: Instant = Instant::now();
        let stale_after: Duration = Duration::from_secs(4);
        let max_skew: Duration = Duration::from_secs(3);
        assert_eq!(
            assess_snapshot(
                [None, Some(2)],
                [None, Some(2)],
                Some(now),
                Some(now),
                now,
                stale_after,
                max_skew
            ),
            Ok(None)
        );
        assert_eq!(
            assess_snapshot(
                [Some(1), Some(2)],
                [Some(1), Some(3)],
                Some(now),
                Some(now),
                now,
                stale_after,
                max_skew
            ),
            Ok(None)
        );
        assert_eq!(
            assess_snapshot(
                [Some(1), Some(2)],
                [Some(1), Some(2)],
                Some(now - Duration::from_secs(5)),
                Some(now),
                now,
                stale_after,
                max_skew
            )
            .expect("valid")
            .expect("unchanged")
            .status,
            HealthStatus::Stale
        );
    }

    #[test]
    fn converts_exchange_seconds_milliseconds_and_rfc3339_consistently() {
        let now: Instant = Instant::now();
        let epoch: f64 = 1_700_000_000.0_f64;
        let expected: Instant = now - Duration::from_secs(2);
        for timestamp in [
            EventTimestamp::UnixSeconds(epoch - 2.0_f64),
            EventTimestamp::UnixMilliseconds((epoch - 2.0_f64) * 1000.0_f64),
            EventTimestamp::Rfc3339("2023-11-14T22:13:18Z"),
            EventTimestamp::Rfc3339("2023-11-15T00:13:18+02:00"),
        ] {
            assert_eq!(event_to_monotonic(timestamp, epoch, now), Ok(expected));
        }
    }

    #[test]
    fn rejects_invalid_nonfinite_future_and_overflowing_timestamps() {
        let now: Instant = Instant::now();
        let epoch: f64 = 1_700_000_000.0_f64;
        assert_eq!(
            event_to_monotonic(EventTimestamp::Rfc3339("bad"), epoch, now),
            Err(HealthError::InvalidTimestamp)
        );
        assert_eq!(
            event_to_monotonic(EventTimestamp::UnixSeconds(f64::NAN), epoch, now),
            Err(HealthError::InvalidTimestamp)
        );
        assert_eq!(
            event_to_monotonic(EventTimestamp::UnixSeconds(epoch + 6.0_f64), epoch, now),
            Err(HealthError::FutureTimestamp)
        );
        assert_eq!(
            event_to_monotonic(EventTimestamp::UnixSeconds(-f64::MAX), epoch, now),
            Err(HealthError::TimestampOutOfRange)
        );
        assert_eq!(
            event_to_monotonic(EventTimestamp::UnixSeconds(epoch + 5.0_f64), epoch, now),
            Ok(now + Duration::from_secs(5))
        );
    }
}
