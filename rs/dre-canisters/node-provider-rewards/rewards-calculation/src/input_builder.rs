use crate::types::{DayEndNanos, RewardPeriod, RewardPeriodError, TimestampNanos};
use ic_base_types::{NodeId, SubnetId};
use ic_protobuf::registry::node_rewards::v2::NodeRewardsTable;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::fmt;

pub struct RewardsCalculatorInput {
    pub reward_period: RewardPeriod,
    pub rewards_table: NodeRewardsTable,
    pub daily_metrics_by_node: BTreeMap<NodeId, Vec<NodeMetricsDaily>>,
    pub daily_subnets_fr: BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>>,
}

#[derive(Default)]
pub struct RewardsCalculatorInputBuilder {
    start_ts: Option<TimestampNanos>,
    end_ts: Option<TimestampNanos>,
    reward_period: Option<RewardPeriod>,
    rewards_table: Option<NodeRewardsTable>,
    daily_metrics_by_node: Option<BTreeMap<NodeId, Vec<NodeMetricsDaily>>>,
}

/// The percentile used to calculate the failure rate for a subnet.
const SUBNET_FAILURE_RATE_PERCENTILE: f64 = 0.75;

impl RewardsCalculatorInputBuilder {
    pub fn with_start_end_ts(mut self, start_ts: TimestampNanos, end_ts: TimestampNanos) -> Self {
        self.start_ts = Some(start_ts);
        self.end_ts = Some(end_ts);
        self
    }

    pub fn with_reward_period(mut self, reward_period: RewardPeriod) -> Self {
        self.reward_period = Some(reward_period);
        self
    }

    pub fn with_rewards_table(mut self, rewards_table: NodeRewardsTable) -> Self {
        self.rewards_table = Some(rewards_table);
        self
    }

    pub fn with_daily_metrics_by_node(mut self, daily_metrics_by_node: BTreeMap<NodeId, Vec<NodeMetricsDaily>>) -> Self {
        self.daily_metrics_by_node = Some(daily_metrics_by_node);
        self
    }

    /// Computes the failure rates for each subnet on a given day.
    ///
    /// The failure rate for a subnet on a given day is defined as the `SUBNET_FAILURE_RATE_PERCENTILE`
    /// of the failure rates of all nodes assigned to that subnet. Days with no recorded
    /// metrics for a subnet are omitted.
    fn subnets_failure_rates(metrics_by_node: &BTreeMap<NodeId, Vec<NodeMetricsDaily>>) -> BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>> {
        let mut rates_map: BTreeMap<(SubnetId, DayEndNanos), Vec<Decimal>> = BTreeMap::new();

        // Aggregate failure rates by (subnet, timestamp)
        for metrics in metrics_by_node.values().flatten() {
            rates_map
                .entry((metrics.subnet_assigned, metrics.ts))
                .or_default()
                .push(metrics.failure_rate);
        }

        let mut subnets_failure_rates: BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>> = BTreeMap::new();

        for ((subnet, ts), mut rates) in rates_map {
            rates.sort();

            let index = ((rates.len() as f64) * SUBNET_FAILURE_RATE_PERCENTILE).ceil() as usize - 1;
            let percentile_rate = SubnetDailyFailureRate { ts, value: rates[index] };

            subnets_failure_rates.entry(subnet).or_default().push(percentile_rate);
        }

        subnets_failure_rates
    }

    pub fn build(self) -> Result<RewardsCalculatorInput, RewardCalculationError> {
        let daily_metrics_by_node = self.daily_metrics_by_node.ok_or("daily_metrics_by_node")?;

        let reward_period = if let Some(reward_period) = self.reward_period {
            reward_period
        } else {
            let start_ts = self.start_ts.ok_or("start_ts")?;
            let end_ts = self.end_ts.ok_or("end_ts")?;
            RewardPeriod::new(start_ts, end_ts)?
        };

        validate_input(&reward_period, &daily_metrics_by_node)?;

        let daily_subnets_fr = Self::subnets_failure_rates(&daily_metrics_by_node);
        let rewards_table = self.rewards_table.ok_or("rewards_table")?;

        Ok(RewardsCalculatorInput {
            reward_period,
            rewards_table,
            daily_metrics_by_node,
            daily_subnets_fr,
        })
    }
}

fn validate_input(reward_period: &RewardPeriod, metrics_by_node: &BTreeMap<NodeId, Vec<NodeMetricsDaily>>) -> Result<(), RewardCalculationError> {
    for (node_id, metrics_entries) in metrics_by_node {
        for entry in metrics_entries {
            // Check if all metrics are within the reward period
            if !reward_period.contains(entry.ts.get()) {
                return Err(RewardCalculationError::NodeMetricsOutOfRange {
                    node_id: *node_id,
                    timestamp: entry.ts.get(),
                    reward_period: reward_period.clone(),
                });
            }
        }
        // Metrics are unique if there are no duplicate entries for the same day and subnet.
        // Metrics with the same timestamp and different subnet are allowed.
        let unique_timestamp_subnet = metrics_entries
            .iter()
            .map(|daily_metrics| (daily_metrics.ts.get(), daily_metrics.subnet_assigned))
            .collect::<HashSet<_>>();
        if unique_timestamp_subnet.len() != metrics_entries.len() {
            return Err(RewardCalculationError::DuplicateMetrics(*node_id));
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum RewardCalculationError {
    RewardPeriodError(RewardPeriodError),
    MissingRequiredField(String),
    EmptyNodes,
    NodeNotInRewardables(NodeId),
    NodeMetricsOutOfRange {
        node_id: NodeId,
        timestamp: TimestampNanos,
        reward_period: RewardPeriod,
    },
    DuplicateMetrics(NodeId),
}

impl From<RewardPeriodError> for RewardCalculationError {
    fn from(err: RewardPeriodError) -> Self {
        RewardCalculationError::RewardPeriodError(err)
    }
}

impl From<&str> for RewardCalculationError {
    fn from(field: &str) -> Self {
        RewardCalculationError::MissingRequiredField(field.to_string())
    }
}

impl Error for RewardCalculationError {}

impl fmt::Display for RewardCalculationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RewardCalculationError::MissingRequiredField(name) => {
                write!(f, "Field {} required", name)
            }
            RewardCalculationError::EmptyNodes => {
                write!(f, "No rewardable nodes provided")
            }
            RewardCalculationError::NodeNotInRewardables(node_id) => {
                write!(f, "Node {} has metrics but it is not part of rewardable nodes", node_id)
            }
            RewardCalculationError::NodeMetricsOutOfRange {
                node_id,
                timestamp,
                reward_period,
            } => {
                write!(
                    f,
                    "Node {} has metrics outside the reward period: timestamp: {} not in {}",
                    node_id, timestamp, reward_period
                )
            }
            RewardCalculationError::DuplicateMetrics(node_id) => {
                write!(f, "Node {} has multiple metrics for the same day", node_id)
            }
            RewardCalculationError::RewardPeriodError(err) => {
                write!(f, "Reward period error: {}", err)
            }
        }
    }
}

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

impl fmt::Display for NodeMetricsDaily {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "blocks_proposed: {}, blocks_failed: {}, failure_rate: {}",
            self.num_blocks_proposed, self.num_blocks_failed, self.failure_rate
        )
    }
}

impl NodeMetricsDaily {
    /// Constructs a new set of daily metrics for a node.
    pub fn new(ts: TimestampNanos, subnet_assigned: SubnetId, num_blocks_proposed: u64, num_blocks_failed: u64) -> Self {
        let total_blocks = Decimal::from(num_blocks_proposed + num_blocks_failed);
        let failure_rate = if total_blocks == Decimal::ZERO {
            Decimal::ZERO
        } else {
            let num_blocks_failed = Decimal::from(num_blocks_failed);
            num_blocks_failed / total_blocks
        };

        NodeMetricsDaily {
            ts: DayEndNanos::from(ts),
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
    pub ts: DayEndNanos,
    pub value: NodeFailureRate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubnetDailyFailureRate {
    pub ts: DayEndNanos,
    pub value: Decimal,
}

#[cfg(test)]
mod tests;
