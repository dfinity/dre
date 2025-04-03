use crate::input_builder::{NodeDailyFailureRate, NodeMetricsDaily};
use ic_base_types::NodeId;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeCategory {
    pub region: String,
    pub node_type: String,
}

#[derive(Default)]
pub struct NodeResults {
    pub region: String,
    pub node_type: String,
    pub daily_metrics: Vec<NodeMetricsDaily>,
    pub daily_fr: Vec<NodeDailyFailureRate>,
    pub average_relative_fr: Decimal,
    pub average_extrapolated_fr: Decimal,
    pub rewards_reduction: Decimal,
    pub performance_multiplier: Decimal,
    pub base_rewards: Decimal,
    pub adjusted_rewards: Decimal,
}

#[derive(Default)]
pub struct RewardsCalculatorResults {
    pub nodes_results: BTreeMap<NodeId, NodeResults>,
    pub rewards_by_category: BTreeMap<NodeCategory, Decimal>,
    pub extrapolated_fr: Decimal,
    pub rewards_total: Decimal,
}
