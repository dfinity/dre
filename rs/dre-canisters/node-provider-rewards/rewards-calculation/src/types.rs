use crate::rewards_calculator_results::{DayUTC, NodeCategory};
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use ic_types::Time;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fmt::Display;

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
// Used to ensure that the wrapped timestamp is aligned to the end of the day.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Default)]
pub struct DayEndNanos(TimestampNanos);

impl From<TimestampNanos> for DayEndNanos {
    fn from(ts: TimestampNanos) -> Self {
        Self(((ts / NANOS_PER_DAY) + 1) * NANOS_PER_DAY - 1)
    }
}

impl DayEndNanos {
    pub fn get(&self) -> TimestampNanos {
        self.0
    }
}

/// Reward period in which we want to reward the node providers
///
/// This period ensures that all `BlockmakerMetrics` collected during the reward period are included consistently
/// with the invariant defined in [`ic_replicated_state::metadata_state::BlockmakerMetricsTimeSeries`].
#[derive(Debug, Clone, PartialEq)]
pub struct RewardPeriod {
    pub from: DayUTC,
    pub to: DayUTC,
}

impl Display for RewardPeriod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RewardPeriod: {} - {}", self.from.ts_at_day_start(), self.to.ts_at_day_end())
    }
}

impl RewardPeriod {
    /// Creates a new `RewardPeriod` from two unaligned timestamps.
    ///
    /// # Arguments
    /// * `unaligned_start_ts` - A generic timestamp (in nanoseconds) in the first (UTC) day.
    /// * `unaligned_end_ts` - A generic timestamp (in nanoseconds) in the last (UTC) day.
    pub fn new(unaligned_start_ts: TimestampNanos, unaligned_end_ts: TimestampNanos) -> Result<Self, RewardPeriodError> {
        if unaligned_start_ts > unaligned_end_ts {
            return Err(RewardPeriodError::StartTimestampAfterEndTimestamp);
        }
        let start_ts: DayEndNanos = unaligned_start_ts.into();
        let end_ts: DayEndNanos = unaligned_end_ts.into();

        // Metrics are collected at the end of the day, so we need to ensure that
        // the end timestamp is not later than the first ts of today.
        let today: DayEndNanos = current_time().as_nanos_since_unix_epoch().into();

        if end_ts.0 >= today.0 {
            return Err(RewardPeriodError::EndTimestampLaterThanToday);
        }

        Ok(Self {
            from: start_ts.into(),
            to: end_ts.into(),
        })
    }

    pub fn contains(&self, day: DayUTC) -> bool {
        day >= self.from && day <= self.to
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

#[derive(Default)]
pub struct ProviderRewardableNodes {
    pub provider_id: PrincipalId,
    pub rewardable_count_by_node_category: HashMap<NodeCategory, usize>,
    pub rewardable_nodes: Vec<RewardableNode>,
}

#[derive(Eq, Hash, PartialEq, Clone, Ord, PartialOrd, Debug)]
pub struct RewardableNode {
    pub node_id: NodeId,
    pub rewardable_days: usize,
    pub region: String,
    pub node_type: String,
    pub dc_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeMetricsDailyRaw {
    pub node_id: NodeId,
    pub num_blocks_proposed: u64,
    pub num_blocks_failed: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct SubnetMetricsDailyKey {
    pub subnet_id: SubnetId,
    pub day: DayUTC,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rewards_calculator_results::days_between;
    use crate::types::TimestampNanos;
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
        let days = days_between(rp.from, rp.to);

        assert_eq!(rp.from.ts_at_day_start(), expected_start_ts);
        assert_eq!(rp.to.ts_at_day_end(), expected_end_ts);
        assert_eq!(days, 4);

        let unaligned_end_ts = ymdh_to_ts(2020, 1, 12, 13);

        let rp = RewardPeriod::new(unaligned_start_ts, unaligned_end_ts).unwrap();
        let days = days_between(rp.from, rp.to);

        assert_eq!(days, 1);
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
