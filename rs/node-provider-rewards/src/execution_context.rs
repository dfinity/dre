use crate::execution_context::performance_multipliers_calculator::StartPerformanceCalculator;
use crate::execution_context::results_tracker::{ResultsTracker, SingleResult};
use crate::execution_context::rewards_calculator::StartRewardsCalculator;
use crate::metrics::{NodeDailyFailureRate, SubnetDailyFailureRate};
use crate::tabled::failure_rates_tabled;
use crate::types::RewardableNode;
use ic_base_types::{NodeId, SubnetId};
use ic_protobuf::registry::node_rewards::v2::NodeRewardsTable;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use tabled::Table;

pub type XDRPermyriad = u64;

pub struct PerformanceCalculatorContext<'a, T: ExecutionState> {
    subnets_fr: &'a BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>>,
    execution_nodes_fr: BTreeMap<NodeId, Vec<NodeDailyFailureRate>>,
    results_tracker: ResultsTracker,
    _marker: PhantomData<T>,
}

impl<'a, T: ExecutionState> PerformanceCalculatorContext<'a, T> {
    pub fn transition<S: ExecutionState>(self) -> PerformanceCalculatorContext<'a, S> {
        PerformanceCalculatorContext {
            subnets_fr: self.subnets_fr,
            execution_nodes_fr: self.execution_nodes_fr,
            results_tracker: self.results_tracker,
            _marker: PhantomData,
        }
    }
}

pub struct RewardsCalculatorContext<'a, T: ExecutionState> {
    rewards_table: &'a NodeRewardsTable,
    provider_nodes: Vec<RewardableNode>,
    results_tracker: ResultsTracker,
    _marker: PhantomData<T>,
}

impl<'a, T: ExecutionState> RewardsCalculatorContext<'a, T> {
    pub fn transition<S: ExecutionState>(self) -> RewardsCalculatorContext<'a, S> {
        RewardsCalculatorContext {
            rewards_table: self.rewards_table,
            provider_nodes: self.provider_nodes,
            results_tracker: self.results_tracker,
            _marker: PhantomData,
        }
    }
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
        let nodes_ids = nodes_ids(&provider_nodes);
        let execution_nodes_fr = self
            .nodes_fr
            .iter()
            .filter(|(node_id, _)| nodes_ids.contains(node_id))
            .map(|(node_id, failure_rates)| (*node_id, failure_rates.clone()))
            .collect();

        // Performance Multipliers Calculation
        let ctx: PerformanceCalculatorContext<StartPerformanceCalculator> = PerformanceCalculatorContext {
            subnets_fr: &self.subnets_fr,
            execution_nodes_fr,
            results_tracker: ResultsTracker::default(),
            _marker: PhantomData,
        };

        let ctx = ctx.next();
        let ctx = ctx.next();
        let ctx = ctx.next();
        let ctx = ctx.next();
        let ctx = ctx.next();
        let ctx: PerformanceCalculatorContext<PerformanceMultipliersComputed> = ctx.next();

        let execution_nodes_fr = ctx.execution_nodes_fr;

        // Rewards Calculation
        let ctx: RewardsCalculatorContext<StartRewardsCalculator> = RewardsCalculatorContext {
            rewards_table: &self.rewards_table,
            provider_nodes,
            results_tracker: ctx.results_tracker,
            _marker: PhantomData,
        };

        let ctx = ctx.next();
        let ctx = ctx.next();
        let ctx = ctx.next();
        let ctx: RewardsCalculatorContext<RewardsTotalComputed> = ctx.next();

        self.post_process(ctx, execution_nodes_fr)
    }
}

pub fn nodes_ids(rewardable_nodes: &[RewardableNode]) -> Vec<NodeId> {
    rewardable_nodes.iter().map(|node| node.node_id).collect()
}

pub fn avg(values: &[Decimal]) -> Decimal {
    values.iter().sum::<Decimal>() / Decimal::from(values.len().max(1))
}

pub struct PerformanceMultipliersComputed;
pub struct RewardsTotalComputed;

pub trait ExecutionState {}
impl ExecutionState for PerformanceMultipliersComputed {}
impl ExecutionState for RewardsTotalComputed {}

mod performance_multipliers_calculator;
pub mod results_tracker;
mod rewards_calculator;
