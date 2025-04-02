use crate::calculation_results::NodeProviderCalculationResults;
use crate::execution_context::performance_multipliers_calculator::{PerformanceCalculatorContext, StartPerformanceCalculator};
use crate::execution_context::rewards_calculator::{RewardsCalculatorContext, StartRewardsCalculator};
use crate::metrics::{NodeDailyFailureRate, SubnetDailyFailureRate};
use crate::types::RewardableNode;
use ic_base_types::{NodeId, SubnetId};
use ic_protobuf::registry::node_rewards::v2::NodeRewardsTable;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashSet};
use std::marker::PhantomData;

pub fn nodes_ids(rewardable_nodes: &[RewardableNode]) -> Vec<NodeId> {
    rewardable_nodes.iter().map(|node| node.node_id).collect()
}

pub fn avg(values: &[Decimal]) -> Decimal {
    values.iter().sum::<Decimal>() / Decimal::from(values.len().max(1))
}

#[derive(Default)]
pub struct ExecutionContext {
    pub subnets_fr: BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>>,
    nodes_fr: BTreeMap<NodeId, Vec<NodeDailyFailureRate>>,
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

    pub fn calculate_rewards(&self, provider_nodes: Vec<RewardableNode>) -> NodeProviderCalculationResults {
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
            calculation_results: NodeProviderCalculationResults::default(),
            _marker: PhantomData,
        };
        let perf_mul_computed: PerformanceCalculatorContext<PerformanceMultipliersComputed> = ctx.next().next().next().next().next().next();

        let (calculation_results, execution_nodes_fr) = (perf_mul_computed.calculation_results, perf_mul_computed.execution_nodes_fr);

        // Rewards Calculation.
        let ctx: RewardsCalculatorContext<StartRewardsCalculator> = RewardsCalculatorContext {
            rewards_table: &self.rewards_table,
            provider_nodes,
            calculation_results,
            _marker: PhantomData,
        };
        let rewards_total_computed: RewardsCalculatorContext<RewardsTotalComputed> = ctx.next().next().next().next();

        let mut calculation_results = rewards_total_computed.calculation_results;

        calculation_results.provider_nodes = rewards_total_computed.provider_nodes;
        calculation_results.nodes_fr = execution_nodes_fr;

        calculation_results
    }
}

pub struct PerformanceMultipliersComputed;
pub struct RewardsTotalComputed;

pub trait ExecutionState {}
impl ExecutionState for PerformanceMultipliersComputed {}
impl ExecutionState for RewardsTotalComputed {}

mod performance_multipliers_calculator;
mod rewards_calculator;
