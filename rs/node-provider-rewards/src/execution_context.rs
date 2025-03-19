use crate::metrics::NodeDailyFailureRate;
use crate::tabled::{failure_rates_tabled, NodesComputationTableBuilder, NodesComputationTabledResult};
use crate::types::RewardableNode;
use ic_base_types::NodeId;
use indexmap::IndexMap;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use tabled::Table;

pub type XDRPermyriad = u64;

#[derive(Eq, Hash, PartialEq, Clone, Copy)]
pub enum NodeResult {
    AverageRelativeFR,
    AverageExtrapolatedFR,
    PerformanceMultiplier,
    RewardsReduction,
    BaseRewards,
    AdjustedRewards,
}

#[derive(Eq, Hash, PartialEq, Clone, Copy)]
pub enum SingleResult {
    ExtrapolatedFR,
    RewardsTotal,
}

impl From<NodeResult> for ResultKey {
    fn from(key: NodeResult) -> Self {
        ResultKey::NR(key)
    }
}

impl From<SingleResult> for ResultKey {
    fn from(key: SingleResult) -> Self {
        ResultKey::SR(key)
    }
}

#[derive(Eq, Hash, PartialEq)]
pub enum ResultKey {
    NR(NodeResult),
    SR(SingleResult),
}

#[derive(Clone)]
enum ResultValue {
    NR(BTreeMap<NodeId, Decimal>),
    SR(Decimal),
}

impl ResultValue {
    pub fn insert_nr(&mut self, node_id: NodeId, value: Decimal) {
        if let ResultValue::NR(map) = self {
            map.insert(node_id, value);
        }
    }

    pub fn insert_sr(&mut self, value: Decimal) {
        if let ResultValue::SR(val) = self {
            *val = value;
        }
    }
}

impl ResultKey {
    pub fn description(&self) -> &'static str {
        match self {
            ResultKey::NR(inner) => match inner {
                NodeResult::AverageRelativeFR => "Average Relative Failure Rate [ARFR]: AVG(RFR(Assigned Days))\n",
                NodeResult::AverageExtrapolatedFR => "Average Extrapolated Failure Rate [AEFR]: AVG(RFR(Assigned Days), EFR(Unassigned Days))\n",
                NodeResult::RewardsReduction => {
                    r#"Rewards Reduction [RR]:
                    * For nodes with AEFR < 0.1, the rewards reduction is 0
                    * For nodes with AEFR > 0.6, the rewards reduction is 0.8
                    * For nodes with 0.1 <= AEFR <= 0.6, the rewards reduction is linearly interpolated between 0 and 0.8
                    "#
                }
                NodeResult::PerformanceMultiplier => "Performance Multiplier [PM]: 1 - RR\n",
                NodeResult::BaseRewards => "Base Rewards\n",
                NodeResult::AdjustedRewards => "Adjusted Rewards: Base Rewards * PM\n",
            },
            ResultKey::SR(inner) => match inner {
                SingleResult::ExtrapolatedFR => "Extrapolated Failure Rate [EFR]: AVG(ARFR)\n",
                SingleResult::RewardsTotal => "Rewards Total\n",
            },
        }
    }
}

#[derive(Default)]
pub struct ExecutionContext<T: ExecutionState> {
    pub provider_nodes: Vec<RewardableNode>,
    pub nodes_failure_rates: BTreeMap<NodeId, Vec<NodeDailyFailureRate>>,
    pub results_tracker: ResultsTracker,
    _marker: PhantomData<T>,
}

#[derive(Default)]
pub struct ResultsTracker(IndexMap<ResultKey, ResultValue>);

impl ResultsTracker {
    fn contains(&self, key: ResultKey) -> bool {
        self.0.contains_key(&key)
    }

    fn get_nodes_result(&self, key: NodeResult) -> Result<BTreeMap<NodeId, Decimal>, String> {
        if let Some(ResultValue::NR(value)) = self.0.get(&ResultKey::NR(key)).cloned() {
            Ok(value)
        } else {
            Err("No ResultValue".to_string())
        }
    }

    fn get_single_result(&self, key: SingleResult) -> Result<Decimal, String> {
        if let Some(ResultValue::SR(value)) = self.0.get(&ResultKey::SR(key)).cloned() {
            Ok(value)
        } else {
            Err("No ResultValue".to_string())
        }
    }

    pub fn record_node_result(&mut self, key: NodeResult, node_id: &NodeId, value: &Decimal) {
        self.0
            .entry(key.into())
            .or_insert(ResultValue::NR(BTreeMap::new()))
            .insert_nr(*node_id, *value);
    }

    pub fn record_single_result(&mut self, key: SingleResult, value: &Decimal) {
        self.0.entry(key.into()).or_insert(ResultValue::SR(*value)).insert_sr(*value);
    }

    fn results_tabled(&self, provider_nodes: Vec<RewardableNode>) -> Vec<Table> {
        let mut builder = NodesComputationTableBuilder::new(provider_nodes);

        for (key, value) in &self.0 {
            match (key, value) {
                (ResultKey::NR(node_result), ResultValue::NR(results_by_node)) => {
                    builder.with_node_result_column(*node_result, results_by_node.clone());
                }
                (ResultKey::SR(single_result), ResultValue::SR(value)) => {
                    builder.with_single_result_column(*single_result, *value);
                }
                _ => panic!("unexpected intermediate result"),
            }
        }
        let NodesComputationTabledResult { legend, computation } = builder.build();

        vec![legend, computation]
    }
}
impl<T: ExecutionState> ExecutionContext<T> {
    pub fn transition<S: ExecutionState>(self) -> ExecutionContext<S> {
        ExecutionContext {
            provider_nodes: self.provider_nodes,
            nodes_failure_rates: self.nodes_failure_rates,
            results_tracker: self.results_tracker,
            _marker: PhantomData,
        }
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
    pub fn next(self) -> Result<ExecutionContext<PerformanceMultipliersComputed>, String> {
        if self.results_tracker.contains(NodeResult::PerformanceMultiplier.into()) {
            Ok(ExecutionContext::transition(self))
        } else {
            Err("Performance multipliers not computed".to_string())
        }
    }
}

impl ExecutionContext<PerformanceMultipliersComputed> {
    // PerformanceMultipliersComputed -> RewardsTotalComputed
    pub fn next(self) -> Result<ExecutionContext<RewardsTotalComputed>, String> {
        if self.results_tracker.contains(SingleResult::RewardsTotal.into()) {
            Ok(ExecutionContext::transition(self))
        } else {
            Err("Performance multipliers not computed".to_string())
        }
    }

    pub fn performance_multipliers(&self) -> BTreeMap<NodeId, Decimal> {
        self.results_tracker
            .get_nodes_result(NodeResult::PerformanceMultiplier)
            .expect("Performance multipliers exist")
    }
}

impl ExecutionContext<RewardsTotalComputed> {
    pub fn computation_tabled(&self) -> Vec<Table> {
        let mut computation_tabled = self.results_tracker.results_tabled(self.provider_nodes.clone());
        computation_tabled.extend(failure_rates_tabled(&self.nodes_failure_rates));
        computation_tabled
    }

    pub fn rewards_total(&self) -> Decimal {
        self.results_tracker
            .get_single_result(SingleResult::RewardsTotal)
            .expect("RewardsTotal exists")
    }
}

pub fn nodes_ids(rewardable_nodes: &[RewardableNode]) -> Vec<NodeId> {
    rewardable_nodes.iter().map(|node| node.node_id).collect()
}

pub fn avg(values: &[Decimal]) -> Decimal {
    values.iter().sum::<Decimal>() / Decimal::from(values.len().max(1))
}
