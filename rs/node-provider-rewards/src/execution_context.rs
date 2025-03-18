use crate::intermediate_results::IntermediateResultsTracker;
use crate::metrics::NodeDailyFailureRate;
use crate::npr_utils::RewardableNode;
use crate::tabled::failure_rates_tabled;
use ic_base_types::NodeId;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use tabled::Table;

pub type XDRPermyriad = u64;

#[derive(Default)]
pub struct ExecutionContext<T: ExecutionState> {
    pub tracker: IntermediateResultsTracker,
    pub provider_nodes: Vec<RewardableNode>,
    pub nodes_failure_rates: BTreeMap<NodeId, Vec<NodeDailyFailureRate>>,
    pub performance_multiplier_by_node: BTreeMap<NodeId, Decimal>,
    pub rewards_total: XDRPermyriad,
    pub _marker: PhantomData<T>,
}
impl<T: ExecutionState> ExecutionContext<T> {
    pub fn transition<S: ExecutionState>(self) -> ExecutionContext<S> {
        ExecutionContext {
            tracker: self.tracker,
            provider_nodes: self.provider_nodes,
            performance_multiplier_by_node: self.performance_multiplier_by_node,
            nodes_failure_rates: self.nodes_failure_rates,
            rewards_total: self.rewards_total,
            _marker: PhantomData,
        }
    }
    pub fn computation_tabled(&self) -> Vec<Table> {
        let mut tables = Vec::new();

        let nodes_computation = self.tracker.nodes_computation_tabled(self.provider_nodes.clone());

        tables.extend(failure_rates_tabled(&self.nodes_failure_rates));
        tables.extend(vec![nodes_computation.legend, nodes_computation.computation]);
        tables
    }
}

pub trait ExecutionState {}

#[derive(Default)]
pub struct Initialized;
pub struct NodesFRInitialized;
pub struct RelativeFRComputed;
pub struct UndefinedFRExtrapolated;
pub struct PerformanceMultipliersComputed;
pub struct RewardsTotalComputed;

impl ExecutionState for Initialized {}
impl ExecutionState for NodesFRInitialized {}
impl ExecutionState for RelativeFRComputed {}
impl ExecutionState for UndefinedFRExtrapolated {}
impl ExecutionState for PerformanceMultipliersComputed {}
impl ExecutionState for RewardsTotalComputed {}

impl ExecutionContext<Initialized> {
    // Initialized -> NodesDailyFRComputed
    pub fn next(self) -> ExecutionContext<NodesFRInitialized> {
        ExecutionContext::transition(self)
    }

    pub fn new(provider_nodes: Vec<RewardableNode>) -> Self {
        ExecutionContext {
            provider_nodes,
            _marker: PhantomData,
            ..Default::default()
        }
    }
}

impl ExecutionContext<NodesFRInitialized> {
    // NodesFRInitialized -> RelativeFRComputed
    pub fn next(self) -> ExecutionContext<RelativeFRComputed> {
        ExecutionContext::transition(self)
    }
}

impl ExecutionContext<RelativeFRComputed> {
    // RelativeFRComputed -> UndefinedFRExtrapolated
    pub fn next(self) -> ExecutionContext<UndefinedFRExtrapolated> {
        ExecutionContext::transition(self)
    }
}

impl ExecutionContext<UndefinedFRExtrapolated> {
    // UndefinedFRExtrapolated -> PerformanceMultipliersComputed
    pub fn next(self) -> ExecutionContext<PerformanceMultipliersComputed> {
        ExecutionContext::transition(self)
    }
}

impl ExecutionContext<PerformanceMultipliersComputed> {
    // PerformanceMultipliersComputed -> RewardsTotalComputed
    pub fn next(self) -> ExecutionContext<RewardsTotalComputed> {
        ExecutionContext::transition(self)
    }
}

impl ExecutionContext<RewardsTotalComputed> {}
