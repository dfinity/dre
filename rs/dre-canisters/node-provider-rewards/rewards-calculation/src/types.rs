use ic_base_types::{NodeId, SubnetId};
use ic_types::Time;
use rust_decimal::Decimal;
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
// Used to ensure that the wrapped timestamp is aligned to the start/end of the day.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DayStartNanos(TimestampNanos);
#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct DayEndNanos(TimestampNanos);

impl From<TimestampNanos> for DayStartNanos {
    fn from(ts: TimestampNanos) -> Self {
        Self((ts / NANOS_PER_DAY) * NANOS_PER_DAY)
    }
}

impl From<TimestampNanos> for DayEndNanos {
    fn from(ts: TimestampNanos) -> Self {
        Self(((ts / NANOS_PER_DAY) + 1) * NANOS_PER_DAY - 1)
    }
}

impl DayStartNanos {
    pub fn get(&self) -> TimestampNanos {
        self.0
    }
}

impl DayEndNanos {
    pub fn get(&self) -> TimestampNanos {
        self.0
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
#[derive(Debug, Clone, PartialEq)]
pub struct RewardPeriod {
    pub start_ts: DayStartNanos,
    pub end_ts: DayEndNanos,
}

impl Display for RewardPeriod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RewardPeriod: {} - {}", self.start_ts.get(), self.end_ts.get())
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
        if unaligned_start_ts > unaligned_end_ts {
            return Err(RewardPeriodError::StartTimestampAfterEndTimestamp);
        }

        // Metrics are collected at the end of the day, so we need to ensure that
        // the end timestamp is not later than the first ts of today.
        let today_first_ts: DayStartNanos = current_time().as_nanos_since_unix_epoch().into();
        if unaligned_end_ts >= today_first_ts.get() {
            return Err(RewardPeriodError::EndTimestampLaterThanToday);
        }

        Ok(Self {
            start_ts: unaligned_start_ts.into(),
            end_ts: unaligned_end_ts.into(),
        })
    }

    pub fn contains(&self, ts: TimestampNanos) -> bool {
        ts >= self.start_ts.get() && ts <= self.end_ts.get()
    }

    pub fn days_between(&self) -> u64 {
        ((self.end_ts.get() - self.start_ts.get()) / NANOS_PER_DAY) + 1
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

#[derive(Eq, Hash, PartialEq, Clone, Ord, PartialOrd)]
pub struct RewardableNode {
    pub node_id: NodeId,
    pub region: String,
    pub node_type: String,
}

/// Represents the daily metrics recorded for a node.
#[derive(Clone, PartialEq, Debug)]
pub struct NodeMetricsDaily {
    pub ts: DayEndNanos,
    pub subnet_assigned: SubnetId,
    pub num_blocks_proposed: u64,
    pub num_blocks_failed: u64,
    pub failure_rate: Decimal,
}

impl NodeMetricsDaily {
    /// Constructs a new set of daily metrics for a node.
    pub fn new(ts: DayEndNanos, subnet_assigned: SubnetId, num_blocks_proposed: u64, num_blocks_failed: u64) -> Self {
        let total_blocks = Decimal::from(num_blocks_proposed + num_blocks_failed);
        let failure_rate = if total_blocks == Decimal::ZERO {
            Decimal::ZERO
        } else {
            let num_blocks_failed = Decimal::from(num_blocks_failed);
            num_blocks_failed / total_blocks
        };

        NodeMetricsDaily {
            ts,
            num_blocks_proposed,
            num_blocks_failed,
            subnet_assigned,
            failure_rate,
        }
    }
}

/// Represents the failure rate status of a node for a day.
///
/// The failure rate is a `Decimal` in the range [0, 1]. The variants provide explicit meaning:
/// - `Defined`: A recorded failure rate for a node assigned to a subnet.
/// - `DefinedRelative`: A failure rate adjusted by the subnet’s failure rate.
/// - `Extrapolated`: An extrapolated failure rate used when `Undefined` failure rates are extrapolated.
/// - `Undefined`: Indicates that no metrics were recorded because the node is not assigned to any subnet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeFailureRate {
    Undefined,
    Extrapolated(Decimal),
    Defined(Decimal),
    DefinedRelative(Decimal),
}

#[derive(Clone, PartialEq, Debug)]

pub enum NodeStatus {
    Assigned {
        subnet_assigned: SubnetId,
        num_blocks_proposed: u64,
        num_blocks_failed: u64,
        original_failure_rate: Decimal,
    },
    Unassigned,
}

#[derive(Clone, PartialEq, Debug)]
pub struct NodeMetricsDailyProcessed {
    pub ts: DayEndNanos,
    pub status: NodeStatus,
    pub failure_rate: NodeFailureRate,
}

impl NodeMetricsDailyProcessed {
    pub fn new_unassigned(ts: DayEndNanos) -> Self {
        NodeMetricsDailyProcessed {
            ts,
            status: NodeStatus::Unassigned,
            failure_rate: NodeFailureRate::Undefined,
        }
    }

    pub fn new_assigned(
        ts: DayEndNanos,
        subnet_assigned: SubnetId,
        num_blocks_proposed: u64,
        num_blocks_failed: u64,
        original_failure_rate: Decimal,
    ) -> Self {
        NodeMetricsDailyProcessed {
            ts,
            status: NodeStatus::Assigned {
                subnet_assigned,
                num_blocks_proposed,
                num_blocks_failed,
                original_failure_rate,
            },
            failure_rate: NodeFailureRate::Defined(original_failure_rate),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubnetFailureRateDaily {
    pub ts: DayEndNanos,
    pub value: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
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

        assert_eq!(rp.start_ts.get(), expected_start_ts);
        assert_eq!(rp.end_ts.get(), expected_end_ts);
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
