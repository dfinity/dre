use crate::metrics::NodeDailyFailureRate;
use crate::types::{NodeCategory, RewardableNode};
use ic_base_types::NodeId;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct NodeProviderCalculationResults {
    pub provider_nodes: Vec<RewardableNode>,
    pub nodes_fr: BTreeMap<NodeId, Vec<NodeDailyFailureRate>>,
    pub average_relative_fr: BTreeMap<NodeId, Decimal>,
    pub average_extrapolated_fr: BTreeMap<NodeId, Decimal>,
    pub rewards_reduction: BTreeMap<NodeId, Decimal>,
    pub performance_multiplier: BTreeMap<NodeId, Decimal>,
    pub monthly_base_rewards: BTreeMap<NodeId, Decimal>,
    pub reward_period_base_rewards: BTreeMap<NodeId, Decimal>,
    pub adjusted_rewards: BTreeMap<NodeId, Decimal>,
    pub extrapolated_fr: Decimal,
    pub rewards_total: Decimal,
    pub rewards_by_category: BTreeMap<NodeCategory, Decimal>,
}
