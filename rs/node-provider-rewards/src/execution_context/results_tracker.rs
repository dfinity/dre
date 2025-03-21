use crate::tabled::{NodesComputationTableBuilder, NodesComputationTabledResult};
use crate::types::{NodeCategory, RewardableNode};
use ic_base_types::NodeId;
use indexmap::IndexMap;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use tabled::Table;

#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
pub enum NodeResult {
    AverageRelativeFR,
    AverageExtrapolatedFR,
    PerformanceMultiplier,
    RewardsReduction,
    BaseRewards,
    AdjustedRewards,
}

#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
pub enum SingleResult {
    ExtrapolatedFR,
    RewardsTotal,
}

#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
pub enum NodeCategoryResult {
    RewardsByCategory,
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

impl From<NodeCategoryResult> for ResultKey {
    fn from(key: NodeCategoryResult) -> Self {
        ResultKey::CR(key)
    }
}

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum ResultKey {
    NR(NodeResult),
    SR(SingleResult),
    CR(NodeCategoryResult),
}

#[derive(Clone)]
enum ResultValue {
    NR(BTreeMap<NodeId, Decimal>),
    SR(Decimal),
    CR(BTreeMap<NodeCategory, Decimal>),
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

    pub fn insert_cr(&mut self, category: NodeCategory, value: Decimal) {
        if let ResultValue::CR(map) = self {
            map.insert(category, value);
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
            ResultKey::CR(inner) => match inner {
                NodeCategoryResult::RewardsByCategory => "Node Category: Region - NodeType\n",
            },
        }
    }
}

#[derive(Default)]
pub struct ResultsTracker(IndexMap<ResultKey, ResultValue>);

impl ResultsTracker {
    pub(super) fn get_category_result(&self, key: NodeCategoryResult) -> &BTreeMap<NodeCategory, Decimal> {
        if let Some(ResultValue::CR(value)) = self.0.get(&ResultKey::CR(key)) {
            value
        } else {
            panic!("{:?} already computed", key);
        }
    }

    pub(super) fn get_nodes_result(&self, key: NodeResult) -> &BTreeMap<NodeId, Decimal> {
        if let Some(ResultValue::NR(value)) = self.0.get(&ResultKey::NR(key)) {
            value
        } else {
            panic!("{:?} already computed", key);
        }
    }

    pub(super) fn get_single_result(&self, key: SingleResult) -> &Decimal {
        if let Some(ResultValue::SR(value)) = self.0.get(&ResultKey::SR(key)) {
            value
        } else {
            panic!("{:?} already computed", key);
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

    pub fn record_category_result(&mut self, key: NodeCategoryResult, category: &NodeCategory, value: &Decimal) {
        self.0
            .entry(key.into())
            .or_insert(ResultValue::CR(BTreeMap::new()))
            .insert_cr(category.clone(), *value);
    }

    pub fn results_tabled(&self, provider_nodes: Vec<RewardableNode>) -> Vec<Table> {
        let mut builder = NodesComputationTableBuilder::new(provider_nodes);

        for (key, value) in &self.0 {
            match (key, value) {
                (ResultKey::NR(node_result), ResultValue::NR(results_by_node)) => {
                    builder.with_node_result_column(*node_result, results_by_node.clone());
                }
                (ResultKey::SR(single_result), ResultValue::SR(value)) => {
                    builder.with_single_result_column(*single_result, *value);
                }
                (ResultKey::CR(_), ResultValue::CR(_)) => continue,
                _ => panic!("unexpected intermediate result"),
            }
        }
        let NodesComputationTabledResult { legend, computation } = builder.build();

        vec![legend, computation]
    }
}
