use crate::npr_utils::{myr_xdr, round, RewardableNode};
use crate::tabled::{NodesComputationTableBuilder, NodesComputationTabledResult};
use ic_base_types::NodeId;
use indexmap::IndexSet;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

pub trait IntermediateResultTrait: Eq + Hash + PartialEq + Clone + Copy {
    fn description(&self) -> &'static str;
}
#[derive(Eq, Hash, PartialEq, Clone, Copy)]
pub enum SingleNodeResult {
    AverageRelativeFR,
    AverageExtrapolatedFR,
    PerformanceMultiplier,
    RewardsReduction,
    BaseRewards,
    AdjustedRewards,
}

#[derive(Eq, Hash, PartialEq, Clone, Copy)]
pub enum AllNodesResult {
    ExtrapolatedFR,
    RewardsTotal,
}

impl IntermediateResultTrait for SingleNodeResult {
    fn description(&self) -> &'static str {
        match self {
            SingleNodeResult::AverageRelativeFR => "Average Relative Failure Rate [ARFR]: AVG(RFR(Assigned Days))\n",
            SingleNodeResult::AverageExtrapolatedFR => "Average Extrapolated Failure Rate [AEFR]: AVG(RFR(Assigned Days), EFR(Unassigned Days))\n",
            SingleNodeResult::RewardsReduction => {
                r#"Rewards Reduction [RR]:
    * For nodes with AEFR < 0.1, the rewards reduction is 0
    * For nodes with AEFR > 0.6, the rewards reduction is 0.8
    * For nodes with 0.1 <= AEFR <= 0.6, the rewards reduction is linearly interpolated between 0 and 0.8
    "#
            }
            SingleNodeResult::PerformanceMultiplier => "Performance Multiplier [PM]: 1 - RR\n",
            SingleNodeResult::BaseRewards => "Base Rewards\n",
            SingleNodeResult::AdjustedRewards => "Adjusted Rewards: Base Rewards * PM\n",
        }
    }
}

impl IntermediateResultTrait for AllNodesResult {
    fn description(&self) -> &'static str {
        match self {
            AllNodesResult::ExtrapolatedFR => "Extrapolated Failure Rate [EFR]: AVG(ARFR)\n",
            AllNodesResult::RewardsTotal => "Rewards Total",
        }
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Copy)]
pub enum IntermediateResult {
    SingleNode(SingleNodeResult),
    AllNodes(AllNodesResult),
}

impl From<SingleNodeResult> for IntermediateResult {
    fn from(value: SingleNodeResult) -> Self {
        IntermediateResult::SingleNode(value)
    }
}

impl From<AllNodesResult> for IntermediateResult {
    fn from(value: AllNodesResult) -> Self {
        IntermediateResult::AllNodes(value)
    }
}

impl IntermediateResultTrait for IntermediateResult {
    fn description(&self) -> &'static str {
        match self {
            IntermediateResult::SingleNode(key) => key.description(),
            IntermediateResult::AllNodes(key) => key.description(),
        }
    }
}

#[derive(Default)]
pub struct IntermediateResultsTracker {
    result_single_node: HashMap<SingleNodeResult, BTreeMap<NodeId, Decimal>>,
    result_all_nodes: HashMap<AllNodesResult, Decimal>,

    results_ordered: IndexSet<IntermediateResult>,
}

impl IntermediateResultsTracker {
    pub fn record_node_result(&mut self, key: SingleNodeResult, node_id: &NodeId, value: &Decimal) {
        self.result_single_node
            .entry(key)
            .or_insert_with(|| {
                self.results_ordered.insert(key.into());
                BTreeMap::new()
            })
            .insert(*node_id, *value);
    }

    pub fn record_all_nodes_result(&mut self, key: AllNodesResult, value: &Decimal) {
        self.result_all_nodes.entry(key).or_insert_with(|| {
            self.results_ordered.insert(key.into());
            *value
        });
    }

    pub fn nodes_computation_tabled(&self, nodes: Vec<RewardableNode>) -> NodesComputationTabledResult {
        let mut builder = NodesComputationTableBuilder::new(nodes);

        for result in &self.results_ordered {
            match result {
                IntermediateResult::SingleNode(single_node_key) => {
                    let results_by_node = self
                        .result_single_node
                        .get(single_node_key)
                        .expect("Result not found")
                        .iter()
                        .map(|(node_id, value)| match single_node_key {
                            SingleNodeResult::BaseRewards | SingleNodeResult::AdjustedRewards => (*node_id, myr_xdr(value)),
                            _ => (*node_id, round(value)),
                        })
                        .collect();

                    builder.with_single_node_result_column(*single_node_key, results_by_node);
                }
                IntermediateResult::AllNodes(all_nodes_key) => {
                    let result = self.result_all_nodes.get(all_nodes_key).expect("Result not found");
                    let result = match all_nodes_key {
                        AllNodesResult::RewardsTotal => myr_xdr(result),
                        _ => round(result),
                    };

                    builder.with_all_nodes_result_column(*all_nodes_key, result);
                }
            }
        }

        builder.build()
    }
}
