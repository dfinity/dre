use crate::reward_period::{TimestampNanos, TimestampNanosAtDayEnd};
use ic_base_types::SubnetId;
use num_traits::FromPrimitive;
use rust_decimal::Decimal;
use std::fmt;

/// Represents the daily metrics recorded for a node.
#[derive(Clone, PartialEq, Debug)]
pub struct NodeDailyMetrics {
    pub ts: TimestampNanosAtDayEnd,
    pub subnet_assigned: SubnetId,
    pub num_blocks_proposed: u64,
    pub num_blocks_failed: u64,
    pub failure_rate: Decimal,
}

impl fmt::Display for NodeDailyMetrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "blocks_proposed: {}, blocks_failed: {}, failure_rate: {}",
            self.num_blocks_proposed, self.num_blocks_failed, self.failure_rate
        )
    }
}

impl NodeDailyMetrics {
    /// Constructs a new set of daily metrics for a node.
    pub fn new(ts: TimestampNanos, subnet_assigned: SubnetId, num_blocks_proposed: u64, num_blocks_failed: u64) -> Self {
        let total_blocks = num_blocks_proposed + num_blocks_failed;
        let failure_rate = if total_blocks == 0 {
            Decimal::ZERO
        } else {
            Decimal::from_f64(num_blocks_failed as f64 / total_blocks as f64).unwrap()
        };
        NodeDailyMetrics {
            ts: TimestampNanosAtDayEnd::from(ts),
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
    Defined {
        subnet_assigned: SubnetId,
        value: Decimal,
    },
    DefinedRelative {
        subnet_assigned: SubnetId,
        original_failure_rate: Decimal,
        subnet_failure_rate: Decimal,
        value: Decimal,
    },
    Extrapolated(Decimal),
    Undefined,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeDailyFailureRate {
    pub ts: TimestampNanos,
    pub value: NodeFailureRate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubnetDailyFailureRate {
    pub ts: TimestampNanos,
    pub value: Decimal,
}
