use crate::metrics::NodeDailyFailureRate;
use crate::types::{NodeCategory, RewardableNode};
use ic_base_types::NodeId;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct NodeProviderCalculationResults {
    pub provider_nodes: Vec<RewardableNode>,
    pub nodes_fr: BTreeMap<NodeId, Vec<NodeDailyFailureRate>>,
    // Average Relative Failure Rate [ARFR]: AVG(RFR(Assigned Days))
    pub average_relative_fr: BTreeMap<NodeId, Decimal>,
    // Average Extrapolated Failure Rate [AEFR]: AVG(RFR(Assigned Days), EFR(Unassigned Days))
    pub average_extrapolated_fr: BTreeMap<NodeId, Decimal>,
    // Rewards Reduction [RR]:
    // * For nodes with AEFR < 0.1, the rewards reduction is 0
    // * For nodes with AEFR > 0.6, the rewards reduction is 0.8
    // * For nodes with 0.1 <= AEFR <= 0.6, the rewards reduction is linearly interpolated between 0 and 0.8
    pub rewards_reduction: BTreeMap<NodeId, Decimal>,
    // Performance Multiplier [PM]: 1 - RR
    pub performance_multiplier: BTreeMap<NodeId, Decimal>,
    pub base_rewards: BTreeMap<NodeId, Decimal>,
    // Adjusted Rewards: Base Rewards * PM
    pub adjusted_rewards: BTreeMap<NodeId, Decimal>,
    // Extrapolated Failure Rate [EFR]: AVG(ARFR)
    pub extrapolated_fr: Decimal,
    pub rewards_total: Decimal,
    // Node Category: Region - NodeType
    pub rewards_by_category: BTreeMap<NodeCategory, Decimal>,
}
