use crate::execution_context::performance_multipliers_calculator::{PerformanceCalculatorContext, StartPerformanceCalculator};
use crate::execution_context::results_tracker::{ResultsTracker, SingleResult};
use crate::execution_context::rewards_calculator::{RewardsCalculatorContext, StartRewardsCalculator};
use crate::metrics::{NodeDailyFailureRate, SubnetDailyFailureRate};
use crate::tabled::failure_rates_tabled;
use crate::types::RewardableNode;
use ic_base_types::{NodeId, SubnetId};
use ic_protobuf::registry::node_rewards::v2::NodeRewardsTable;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashSet};
use std::marker::PhantomData;
use tabled::Table;

pub fn nodes_ids(rewardable_nodes: &[RewardableNode]) -> Vec<NodeId> {
    rewardable_nodes.iter().map(|node| node.node_id).collect()
}

pub fn avg(values: &[Decimal]) -> Decimal {
    values.iter().sum::<Decimal>() / Decimal::from(values.len().max(1))
}

pub struct RewardsCalculationResult {
    pub rewards: Decimal,
    pub computation_log_tabled: Vec<Table>,
}

#[derive(Default)]
pub struct ExecutionContext {
    nodes_fr: BTreeMap<NodeId, Vec<NodeDailyFailureRate>>,
    subnets_fr: BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>>,
    rewards_table: NodeRewardsTable,
}

impl ExecutionContext {
    pub fn new(
        nodes_fr: BTreeMap<NodeId, Vec<NodeDailyFailureRate>>,
        subnets_fr: BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>>,
        rewards_table: NodeRewardsTable,
    ) -> Self {
        ExecutionContext {
            nodes_fr,
            subnets_fr,
            rewards_table,
        }
    }

    fn post_process(
        &self,
        ctx: RewardsCalculatorContext<RewardsTotalComputed>,
        execution_nodes_fr: BTreeMap<NodeId, Vec<NodeDailyFailureRate>>,
    ) -> RewardsCalculationResult {
        let mut computation_log_tabled = ctx.results_tracker.results_tabled(ctx.provider_nodes);
        computation_log_tabled.extend(failure_rates_tabled(execution_nodes_fr));
        let rewards = *ctx.results_tracker.get_single_result(SingleResult::RewardsTotal);

        RewardsCalculationResult {
            rewards,
            computation_log_tabled,
        }
    }

    pub fn calculate_rewards(&self, provider_nodes: Vec<RewardableNode>) -> RewardsCalculationResult {
        // HashSetize because looking up a node ID into a HashSet (done below) is O(1).
        let nodes_ids: HashSet<NodeId> = HashSet::from_iter(nodes_ids(&provider_nodes));

        // Performance Multipliers Calculation
        let ctx: PerformanceCalculatorContext<StartPerformanceCalculator> = PerformanceCalculatorContext {
            subnets_fr: &self.subnets_fr,
            execution_nodes_fr: self
                .nodes_fr
                .iter()
                .filter(|(node_id, _)| nodes_ids.contains(node_id))
                .map(|(node_id, failure_rates)| (*node_id, failure_rates.clone()))
                .collect(),
            results_tracker: ResultsTracker::default(),
            _marker: PhantomData,
        };

        // Step through the multiple steps of the calculation.
        let perfmulcomputed: PerformanceCalculatorContext<PerformanceMultipliersComputed> = ctx.next().next().next().next().next().next();

        // Move the values of the resulting calculation outside the container.
        let (results_tracker, execution_nodes_fr) = (perfmulcomputed.results_tracker, perfmulcomputed.execution_nodes_fr);

        // Rewards Calculation.
        let ctx: RewardsCalculatorContext<StartRewardsCalculator> = RewardsCalculatorContext {
            rewards_table: &self.rewards_table,
            provider_nodes,
            results_tracker,
            _marker: PhantomData,
        };

        self.post_process(ctx.next().next().next().next(), execution_nodes_fr)
    }
}

pub struct PerformanceMultipliersComputed;
pub struct RewardsTotalComputed;

pub trait ExecutionState {}
impl ExecutionState for PerformanceMultipliersComputed {}
impl ExecutionState for RewardsTotalComputed {}

mod performance_multipliers_calculator;
pub mod results_tracker;
mod rewards_calculator;
