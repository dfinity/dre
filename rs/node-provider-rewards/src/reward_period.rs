use ic_types::Time;
use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::ops::Deref;

pub type TimestampNanos = u64;
pub const NANOS_PER_DAY: TimestampNanos = 24 * 60 * 60 * 1_000_000_000;

#[cfg(target_arch = "wasm32")]
fn current_time() -> Time {
    let current_time = ic_cdk::api::time();
    Time::from_nanos_since_unix_epoch(current_time)
}

#[cfg(not(any(target_arch = "wasm32")))]
fn current_time() -> Time {
    ic_types::time::current_time()
}

// Wrapper types for TimestampNanos.
// Used to ensure that the wrapped timestamp is aligned to the start/end of the day.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TimestampNanosAtDayStart(TimestampNanos);
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TimestampNanosAtDayEnd(TimestampNanos);

impl From<TimestampNanos> for TimestampNanosAtDayStart {
    fn from(ts: TimestampNanos) -> Self {
        Self((ts / NANOS_PER_DAY) * NANOS_PER_DAY)
    }
}

impl From<TimestampNanos> for TimestampNanosAtDayEnd {
    fn from(ts: TimestampNanos) -> Self {
        Self(((ts / NANOS_PER_DAY) + 1) * NANOS_PER_DAY - 1)
    }
}

impl Deref for TimestampNanosAtDayEnd {
    type Target = TimestampNanos;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Deref for TimestampNanosAtDayStart {
    type Target = TimestampNanos;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Reward period in which we want to reward the node providers
///
/// Reward period spans over two timestamp boundaries:
///  - `start_ts`: The first timestamp (in nanoseconds) of the first day.
///  - `end_ts`: The last timestamp (in nanoseconds) of the last day.
///
/// This period ensures that all `BlockmakerMetrics` collected during the reward period are included consistently
/// with the invariant defined in [`ic_replicated_state::metadata_state::BlockmakerMetricsTimeSeries`].
///
/// TODO: This should be moved to NPR canister crate.
#[derive(Debug, Clone, PartialEq)]
pub struct RewardPeriod {
    pub start_ts: TimestampNanosAtDayStart,
    pub end_ts: TimestampNanosAtDayEnd,
}

impl Display for RewardPeriod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RewardPeriod: {} - {}", *self.start_ts, *self.end_ts)
    }
}

impl RewardPeriod {
    /// Creates a new `RewardPeriod` from two unaligned timestamps.
    ///
    /// # Arguments
    /// * `unaligned_start_ts` - A generic timestamp (in nanoseconds) in the first (UTC) day.
    /// * `unaligned_end_ts` - A generic timestamp (in nanoseconds) in the last (UTC) day.
    ///
    /// # Errors
    /// * `RewardPeriodError::StartTimestampAfterEndTimestamp` - If `unaligned_start_ts` is greater than `unaligned_end_ts`.
    /// * `RewardPeriodError::EndTimestampLaterThanToday` - If `unaligned_end_ts` is later than the first timestamp of today.
    pub fn new(unaligned_start_ts: TimestampNanos, unaligned_end_ts: TimestampNanos) -> Result<Self, RewardPeriodError> {
        if unaligned_start_ts >= unaligned_end_ts {
            return Err(RewardPeriodError::StartTimestampAfterEndTimestamp);
        }

        // Metrics are collected at the end of the day, so we need to ensure that
        // the end timestamp is not later than the first ts of today.
        let today_first_ts: TimestampNanosAtDayStart = current_time().as_nanos_since_unix_epoch().into();
        if unaligned_end_ts >= *today_first_ts {
            return Err(RewardPeriodError::EndTimestampLaterThanToday);
        }

        Ok(Self {
            start_ts: unaligned_start_ts.into(),
            end_ts: unaligned_end_ts.into(),
        })
    }

    pub fn contains(&self, ts: TimestampNanos) -> bool {
        ts >= *self.start_ts && ts <= *self.end_ts
    }

    pub fn days_between(&self) -> u64 {
        ((*self.end_ts - *self.start_ts) / NANOS_PER_DAY) + 1
    }
}

#[derive(Debug, PartialEq)]
pub enum RewardPeriodError {
    StartTimestampAfterEndTimestamp,
    EndTimestampLaterThanToday,
}

impl fmt::Display for RewardPeriodError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RewardPeriodError::StartTimestampAfterEndTimestamp => {
                write!(f, "unaligned_start_ts must be earlier than unaligned_end_ts.")
            }
            RewardPeriodError::EndTimestampLaterThanToday => {
                write!(f, "unaligned_end_ts must be earlier than today")
            }
        }
    }
}

impl Error for RewardPeriodError {}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn ymdh_to_ts(year: i32, month: u32, day: u32, hour: u32) -> TimestampNanos {
        Utc.with_ymd_and_hms(year, month, day, hour, 0, 0).unwrap().timestamp_nanos_opt().unwrap() as TimestampNanos
    }

    #[test]
    fn test_valid_rewarding_period() {
        let unaligned_start_ts = ymdh_to_ts(2020, 1, 12, 12);
        let unaligned_end_ts = ymdh_to_ts(2020, 1, 15, 15);

        let rp = RewardPeriod::new(unaligned_start_ts, unaligned_end_ts).unwrap();
        let expected_start_ts = ymdh_to_ts(2020, 1, 12, 0);
        let expected_end_ts = ymdh_to_ts(2020, 1, 16, 0) - 1;

        assert_eq!(*rp.start_ts, expected_start_ts);
        assert_eq!(*rp.end_ts, expected_end_ts);
        assert_eq!(rp.days_between(), 4);

        let unaligned_end_ts = ymdh_to_ts(2020, 1, 12, 13);

        let rp = RewardPeriod::new(unaligned_start_ts, unaligned_end_ts).unwrap();

        assert_eq!(rp.days_between(), 1);
    }

    #[test]
    fn test_error_too_recent_end_ts() {
        let to_ts = current_time().as_nanos_since_unix_epoch();
        let from_ts = 0;

        let rp = RewardPeriod::new(from_ts, to_ts);
        assert_eq!(rp.unwrap_err(), RewardPeriodError::EndTimestampLaterThanToday);
    }

    #[test]
    fn test_error_unaligned_start_ts_greater_unaligned_end_ts() {
        let to_ts = 0;
        let from_ts = 1;

        let rp = RewardPeriod::new(from_ts, to_ts);

        assert_eq!(rp.unwrap_err(), RewardPeriodError::StartTimestampAfterEndTimestamp);
    }
}
