use crate::rewards_calculator::node_provider_rewards_calculator::{Initialized, NodeProviderRewardsCalculator, RewardsTotalComputed};
use crate::rewards_calculator_results::{RewardCalculatorError, RewardsCalculatorResults};
use crate::types::{NodeMetricsDaily, RewardableNode, SubnetFailureRateDaily};
use crate::types::{RewardPeriod, TimestampNanos};
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use ic_protobuf::registry::node_rewards::v2::NodeRewardsTable;
use itertools::Itertools;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::marker::PhantomData;

/// The percentile used to calculate the failure rate for a subnet.
const SUBNET_FAILURE_RATE_PERCENTILE: f64 = 0.75;

/// Represents the input required for the rewards calculator.
///
/// # Arguments
/// * reward_period - The time frame for which rewards are calculated.
/// * rewards_table - The rewards table containing the reward rates for each node type.
/// * daily_metrics_by_node - Daily node metrics for nodes in `reward_period`.
/// * daily_subnets_fr: Daily Subnets failure rates in `reward_period`.
pub struct RewardsCalculator {
    reward_period: RewardPeriod,
    rewards_table: NodeRewardsTable,
    daily_metrics_by_node: BTreeMap<NodeId, Vec<NodeMetricsDaily>>,
    daily_subnets_fr: RefCell<BTreeMap<SubnetId, Vec<SubnetFailureRateDaily>>>,
}

impl RewardsCalculator {
    pub fn new(
        reward_period: RewardPeriod,
        rewards_table: NodeRewardsTable,
        daily_metrics_by_node: BTreeMap<NodeId, Vec<NodeMetricsDaily>>,
    ) -> Result<Self, RewardCalculatorError> {
        validate_input(&reward_period, &daily_metrics_by_node)?;

        Ok(Self {
            reward_period,
            rewards_table,
            daily_metrics_by_node,
            daily_subnets_fr: RefCell::new(BTreeMap::new()),
        })
    }

    fn compute_daily_subnet_fr(&self, subnet_id: &SubnetId) -> Vec<SubnetFailureRateDaily> {
        self.daily_metrics_by_node
            .values()
            .flatten()
            .filter(|metrics| metrics.subnet_assigned == *subnet_id)
            .into_group_map_by(|metrics| metrics.ts)
            .into_iter()
            .sorted_by_key(|(ts, _)| *ts)
            .map(|(ts, metrics)| {
                let failure_rates = metrics.iter().map(|m| m.failure_rate).collect::<Vec<_>>();
                let index = ((failure_rates.len() as f64) * SUBNET_FAILURE_RATE_PERCENTILE).ceil() as usize - 1;
                SubnetFailureRateDaily {
                    ts,
                    value: failure_rates[index],
                }
            })
            .collect()
    }

    fn get_current_daily_subnets_fr(&self, subnets: HashSet<SubnetId>) -> BTreeMap<SubnetId, Vec<SubnetFailureRateDaily>> {
        let mut current_daily_subnets_fr = BTreeMap::new();
        let mut daily_subnets_fr = self.daily_subnets_fr.borrow_mut();

        for subnet_id in subnets {
            let subnet_fr = daily_subnets_fr
                // Fill the cache with the subnet failure rate if it is not already present
                .entry(subnet_id)
                .or_insert_with(|| self.compute_daily_subnet_fr(&subnet_id));

            current_daily_subnets_fr.insert(subnet_id, subnet_fr.clone());
        }
        current_daily_subnets_fr
    }

    pub fn calculate_node_provider_rewards(
        &self,
        provider_id: PrincipalId,
        rewardable_nodes: Vec<RewardableNode>,
    ) -> Result<RewardsCalculatorResults, RewardCalculatorError> {
        if rewardable_nodes.is_empty() {
            return Err(RewardCalculatorError::EmptyMetrics);
        }

        let nodes_subnets: HashSet<SubnetId> = rewardable_nodes
            .iter()
            .filter_map(|node| {
                self.daily_metrics_by_node
                    .get(&node.node_id)
                    .map(|metrics| metrics.iter().map(|m| m.subnet_assigned).collect::<HashSet<_>>())
            })
            .flatten()
            .collect();
        let daily_subnets_fr = self.get_current_daily_subnets_fr(nodes_subnets);

        let ctx: NodeProviderRewardsCalculator<Initialized> = NodeProviderRewardsCalculator {
            provider_id,
            rewardable_nodes,
            daily_subnets_fr,
            calculator_results: RewardsCalculatorResults::default(),
            reward_period: &self.reward_period,
            rewards_table: &self.rewards_table,
            daily_metrics_by_node: &self.daily_metrics_by_node,
            _marker: PhantomData,
        };
        let computed: NodeProviderRewardsCalculator<RewardsTotalComputed> = ctx.next().next().next().next().next().next().next().next().next().next();
        Ok(computed.get_results())
    }
}

fn validate_input(
    reward_period: &RewardPeriod,
    daily_metrics_by_node: &BTreeMap<NodeId, Vec<NodeMetricsDaily>>,
) -> Result<(), RewardCalculatorError> {
    for (node_id, daily_metrics) in daily_metrics_by_node.iter() {
        for entry in daily_metrics {
            // Check if all metrics are within the reward period
            if !reward_period.contains(entry.ts.get()) {
                return Err(RewardCalculatorError::NodeMetricsOutOfRange {
                    node_id: *node_id,
                    timestamp: entry.ts.get(),
                    reward_period: reward_period.clone(),
                });
            }
        }
        // Metrics are unique if there are no duplicate entries for the same day and subnet.
        // Metrics with the same timestamp and different subnet are allowed.
        let unique_timestamp_subnet = daily_metrics
            .iter()
            .map(|entry| (entry.ts.get(), entry.subnet_assigned))
            .collect::<HashSet<_>>();
        if unique_timestamp_subnet.len() != daily_metrics.len() {
            return Err(RewardCalculatorError::DuplicateMetrics(*node_id));
        }
    }
    Ok(())
}

mod node_provider_rewards_calculator;

#[cfg(test)]
mod tests;
