use crate::types::{DayEndNanos, RewardPeriod, RewardPeriodError, TimestampNanos};
use ic_base_types::{NodeId, SubnetId};
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Default, Clone)]
pub struct XDRPermyriad(Decimal);

impl XDRPermyriad {
    pub fn get(&self) -> Decimal {
        self.0
    }
}
impl From<Decimal> for XDRPermyriad {
    fn from(value: Decimal) -> Self {
        XDRPermyriad(value)
    }
}

#[derive(Clone, PartialEq, Debug, Default)]

pub struct Percent(Decimal);

impl Percent {
    pub fn get(&self) -> Decimal {
        self.0
    }
}

impl From<Decimal> for Percent {
    fn from(value: Decimal) -> Self {
        Percent(value)
    }
}

#[derive(Clone, PartialEq, Debug, Hash, PartialOrd, Ord, Eq, Copy)]
pub struct DayUTC(DayEndNanos);

impl DayUTC {
    pub fn get(&self) -> TimestampNanos {
        self.0.get()
    }
}
impl From<DayEndNanos> for DayUTC {
    fn from(value: DayEndNanos) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeCategory {
    pub region: String,
    pub node_type: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct NodeMetricsDailyProcessed {
    pub day: DayUTC,
    pub subnet_assigned: SubnetId,
    /// Subnet Assigned Failure Rate.
    ///
    /// The failure rate of the entire subnet.
    /// Calculated as 75th percentile of the failure rate of all nodes in the subnet.
    pub subnet_assigned_fr: Percent,
    pub num_blocks_proposed: u64,
    pub num_blocks_failed: u64,
    /// Original Failure Rate.
    ///
    /// The failure rate before subnet failure rate reduction.
    /// Calculated as `blocks_failed` / (`blocks_proposed` + `blocks_failed`)
    pub original_fr: Percent,
    /// Relative Failure Rate (`RFR`).
    ///
    /// The failure rate reduced by the subnet assigned failure rate.
    /// Calculated as Max(0, `original_fr` - `subnet_assigned_fr`)
    pub relative_fr: Percent,
}

#[derive(Debug, Default)]
pub struct NodeResults {
    pub region: String,
    pub node_type: String,
    pub dc_id: String,
    pub daily_metrics: Vec<NodeMetricsDailyProcessed>,

    /// Average Relative Failure Rate (`ARFR`).
    ///
    /// Average of `RFR` for the entire reward period.
    /// None if the node is unassigned in the entire reward period
    pub avg_relative_fr: Option<Percent>,

    /// Average Extrapolated Failure Rate (`AEFR`).
    ///
    /// Failure rate average for the entire reward period
    /// - On days when the node is unassigned `AEFR` is used
    /// - On days when the node is assigned `RFR` is used
    pub avg_extrapolated_fr: Percent,

    /// Rewards reduction (`RR`).
    ///
    /// - For nodes with `AEFR` < 0.1, the rewards reduction is 0
    /// - For nodes with `AEFR` > 0.6, the rewards reduction is 0.8
    /// - For nodes with 0.1 <= `AEFR` <= 0.6, the rewards reduction is linearly interpolated between 0 and 0.8
    pub rewards_reduction: Percent,

    /// Performance multiplier (`PM`).
    ///
    /// Calculated as 1 - 'RR'
    pub performance_multiplier: Percent,
    pub base_rewards: XDRPermyriad,

    /// Adjusted rewards (`AR`).
    ///
    /// Calculated as base_rewards * `PM`
    pub adjusted_rewards: XDRPermyriad,
}

#[derive(Debug, Default)]
pub struct RewardsCalculatorResults {
    pub results_by_node: BTreeMap<NodeId, NodeResults>,
    // [EFR]
    // Extrapolated failure rate used as replacement for days when the node is unassigned
    pub extrapolated_fr: Percent,
    /// Rewards Total
    /// The total rewards for the entire reward period computed as sum of the `AR`
    pub rewards_total: XDRPermyriad,
}

#[derive(Debug, PartialEq)]
pub enum RewardCalculatorError {
    RewardPeriodError(RewardPeriodError),
    EmptyMetrics,
    SubnetMetricsOutOfRange {
        subnet_id: SubnetId,
        timestamp: TimestampNanos,
        reward_period: RewardPeriod,
    },
    DuplicateMetrics(SubnetId, DayEndNanos),
}

impl From<RewardPeriodError> for RewardCalculatorError {
    fn from(err: RewardPeriodError) -> Self {
        RewardCalculatorError::RewardPeriodError(err)
    }
}

impl Error for RewardCalculatorError {}

impl fmt::Display for RewardCalculatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RewardCalculatorError::EmptyMetrics => {
                write!(f, "No daily_metrics_by_node")
            }
            RewardCalculatorError::SubnetMetricsOutOfRange {
                subnet_id,
                timestamp,
                reward_period,
            } => {
                write!(
                    f,
                    "Node {} has metrics outside the reward period: timestamp: {} not in {}",
                    subnet_id, timestamp, reward_period
                )
            }
            RewardCalculatorError::DuplicateMetrics(subnet_id, ts) => {
                write!(f, "Subnet {} has multiple metrics for the same node at ts {}", subnet_id, ts.get())
            }
            RewardCalculatorError::RewardPeriodError(err) => {
                write!(f, "Reward period error: {}", err)
            }
        }
    }
}
