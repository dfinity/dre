use chrono::DateTime;
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use rewards_calculation::calculation_results::RewardsCalculatorResults;
use rewards_calculation::metrics::NodeFailureRate;
use rewards_calculation::types::TimestampNanos;
use rewards_calculation::RewardsCalculationResult;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

#[derive(candid::CandidType, candid::Deserialize)]
pub struct RewardPeriodArgs {
    pub start_ts: u64,
    pub end_ts: u64,
}

#[derive(candid::CandidType, candid::Deserialize)]
pub struct NodeProvidersRewardsXDRTotal {
    pub rewards_per_provider: BTreeMap<PrincipalId, u64>,
}

impl NodeProvidersRewardsXDRTotal {
    pub fn new(result: BTreeMap<PrincipalId, Decimal>) -> Self {
        Self {
            rewards_per_provider: result.into_iter().map(|(k, v)| (k, v.to_u64().unwrap())).collect(),
        }
    }
}

#[derive(candid::CandidType, candid::Deserialize)]
pub struct NodeProviderRewardsCalculationArgs {
    pub provider_id: PrincipalId,
    pub reward_period: RewardPeriodArgs,
}

#[derive(candid::CandidType, candid::Deserialize)]
pub struct DailyNodeResults {
    pub utc_day: String,
    pub subnet_assigned: SubnetId,
    pub blocks_proposed: u64,
    pub blocks_failed: u64,
    pub original_failure_rate: f64,
    pub relative_failure_rate: f64,
}

#[derive(candid::CandidType, candid::Deserialize, Default)]
pub struct NodeResults {
    pub node_type: String,
    pub region: String,
    pub daily_node_results: Option<Vec<DailyNodeResults>>,
    pub average_fr: f64,
    pub rewards_reduction: f64,
    pub performance_multiplier: f64,
    pub base_rewards: f64,
    pub adjusted_rewards: f64,
}

#[derive(candid::CandidType, candid::Deserialize)]
pub struct BaseRewardsByCategory {
    pub node_type: String,
    pub region: String,
    pub base_rewards: f64,
}

#[derive(candid::CandidType, candid::Deserialize)]
pub struct DailySubnetFailureRate {
    pub utc_day: String,
    pub fr: f64,
}

#[derive(candid::CandidType, candid::Deserialize)]
pub struct NodeProviderRewardsCalculation {
    pub daily_subnets_fr: BTreeMap<SubnetId, Vec<DailySubnetFailureRate>>,
    pub extrapolated_fr: f64,
    pub results_by_node: BTreeMap<NodeId, NodeResults>,
    pub rewards_by_category: Vec<BaseRewardsByCategory>,
    pub rewards_total_xdr: u64,
}

impl From<RewardsCalculatorResults> for NodeProviderRewardsCalculation {
    fn from(result: RewardsCalculatorResults) -> Self {
        let mut result = result;
        let RewardsCalculatorResults {
            provider_nodes,
            extrapolated_fr,
            rewards_by_category,
            rewards_total,
            mut nodes_fr,
            mut average_extrapolated_fr,
            rewards_reductions: mut rewards_reduction,
            performance_multipliers: mut performance_multiplier,
            mut base_rewards,
            mut adjusted_rewards,
            ..
        } = result.results_per_provider.pop_first().expect("Exists").1;

        let daily_subnets_fr = result
            .subnets_failure_rates
            .into_iter()
            .map(|(subnet_id, daily_fr)| {
                let subnet_daily_fr = daily_fr
                    .into_iter()
                    .map(move |fr| DailySubnetFailureRate {
                        utc_day: timestamp_to_utc_date(fr.ts.get()),
                        fr: fr.value.round_dp(4).to_f64().unwrap(),
                    })
                    .collect();

                (subnet_id, subnet_daily_fr)
            })
            .collect();

        let extrapolated_fr = extrapolated_fr.round_dp(4).to_f64().unwrap();

        let mut results_by_node = BTreeMap::new();
        for node in provider_nodes.into_iter() {
            let entry: &mut NodeResults = results_by_node.entry(node.node_id).or_default();
            entry.node_type = node.node_type;
            entry.region = node.region;

            let assigned_fr: Vec<DailyNodeResults> = nodes_fr
                .remove(&node.node_id)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|fr| match fr.value {
                    NodeFailureRate::DefinedRelative {
                        subnet_assigned,
                        original_failure_rate,
                        value,
                        ..
                    } => Some(DailyNodeResults {
                        utc_day: timestamp_to_utc_date(fr.ts.get()),
                        subnet_assigned,
                        blocks_proposed: 0,
                        blocks_failed: 0,
                        original_failure_rate: original_failure_rate.round_dp(4).to_f64().unwrap(),
                        relative_failure_rate: value.round_dp(4).to_f64().unwrap(),
                    }),
                    _ => None,
                })
                .collect();

            if !assigned_fr.is_empty() {
                entry.daily_node_results = Some(assigned_fr);
            }

            entry.average_fr = average_extrapolated_fr
                .remove(&node.node_id)
                .expect("Average extrapolated fr exists for all nodes")
                .round_dp(4)
                .to_f64()
                .unwrap();
            entry.rewards_reduction = rewards_reduction
                .remove(&node.node_id)
                .expect("Rewards reduction exists for all nodes")
                .round_dp(4)
                .to_f64()
                .unwrap();
            entry.performance_multiplier = performance_multiplier
                .remove(&node.node_id)
                .expect("Performance multiplier exists for all nodes")
                .round_dp(4)
                .to_f64()
                .unwrap();
            entry.base_rewards = base_rewards
                .remove(&node.node_id)
                .expect("Base Rewards exist for all nodes")
                .round_dp(4)
                .to_f64()
                .unwrap();
            entry.adjusted_rewards = adjusted_rewards
                .remove(&node.node_id)
                .expect("Adjusted Rewards exist for all nodes")
                .round_dp(4)
                .to_f64()
                .unwrap();
        }

        let rewards_by_category = rewards_by_category
            .into_iter()
            .map(|(category, rewards)| BaseRewardsByCategory {
                node_type: category.node_type.to_string(),
                region: category.region.to_string(),
                base_rewards: rewards.round_dp(4).to_f64().unwrap(),
            })
            .collect();

        let rewards_total_xdr = rewards_total.to_u64().unwrap();

        Self {
            daily_subnets_fr,
            extrapolated_fr,
            results_by_node,
            rewards_by_category,
            rewards_total_xdr,
        }
    }
}

fn timestamp_to_utc_date(ts: TimestampNanos) -> String {
    DateTime::from_timestamp(ts as i64 / 1_000_000_000, 0)
        .unwrap()
        .naive_utc()
        .format("%d-%m-%Y")
        .to_string()
}
