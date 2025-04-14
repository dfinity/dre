use chrono::DateTime;
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use itertools::Itertools;
use rewards_calculation::rewards_calculator_results::RewardsCalculatorResults;
use rewards_calculation::types::TimestampNanos;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;

#[derive(candid::CandidType, candid::Deserialize)]
pub struct RewardPeriodArgs {
    // First timestamp of the day in nanoseconds
    pub start_ts: u64,
    // Last timestamp of the day in nanoseconds
    pub end_ts: u64,
}

#[derive(candid::CandidType, candid::Deserialize)]
pub struct NodeProvidersRewardsXDRTotal {
    // Total rewards in permyriad XDR for all providers
    pub rewards_xdr_permyriad_per_provider: BTreeMap<PrincipalId, u64>,
}

impl TryFrom<BTreeMap<PrincipalId, Decimal>> for NodeProvidersRewardsXDRTotal {
    type Error = String;

    fn try_from(result: BTreeMap<PrincipalId, Decimal>) -> Result<Self, Self::Error> {
        let rewards_xdr_permyriad_per_provider = result
            .into_iter()
            .map(|(k, v)| Ok::<(PrincipalId, u64), String>((k, v.to_u64().err_u64()?)))
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        Ok(Self {
            rewards_xdr_permyriad_per_provider,
        })
    }
}

#[derive(candid::CandidType, candid::Deserialize)]
pub struct NodeProviderRewardsCalculationArgs {
    pub provider_id: PrincipalId,
    pub reward_period: RewardPeriodArgs,
}

#[derive(candid::CandidType, candid::Deserialize)]
pub struct DailyNodeResults {
    // UTC Day with format "DD-MM-YYYY"
    pub utc_day: String,
    pub subnet_assigned: SubnetId,
    pub blocks_proposed: u64,
    pub blocks_failed: u64,
    // The failure rate before subnet failure rate adjustment
    pub original_failure_rate: f64,
    // [RFR]
    // The daily failure rate after subnet failure rate adjustment
    pub relative_failure_rate: f64,
}

#[derive(candid::CandidType, candid::Deserialize, Default)]
pub struct NodeResults {
    pub node_type: String,
    pub region: String,
    // None if the node is unassigned in the entire reward period
    pub daily_node_results: Option<Vec<DailyNodeResults>>,
    // [AEFR]
    // Failure rate average for the entire reward period
    // On days when the node is unassigned EFR is used
    // On days when the node is assigned RFR is used
    pub average_fr: f64,
    // [RR]
    // * For nodes with AEFR < 0.1, the rewards reduction is 0
    // * For nodes with AEFR > 0.6, the rewards reduction is 0.8
    // * For nodes with 0.1 <= AEFR <= 0.6, the rewards reduction is linearly interpolated between 0 and 0.8
    pub rewards_reduction: f64,
    // [PM]
    // Performance multiplier is calculated as 1 - RR
    pub performance_multiplier: f64,
    pub base_rewards: f64,
    // [AR]
    // Adjusted rewards are calculated as base_rewards * PM
    pub adjusted_rewards: f64,
}

#[derive(candid::CandidType, candid::Deserialize, Debug)]
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
    // [EFR]
    // Extrapolated failure rate used as replacement for days when the node is unassigned
    pub extrapolated_fr: f64,
    pub results_by_node: BTreeMap<NodeId, NodeResults>,
    pub rewards_by_category: Vec<BaseRewardsByCategory>,
    // Total rewards for the provider in XDR
    pub rewards_total_xdr: u64,
}

impl TryFrom<RewardsCalculatorResults> for NodeProviderRewardsCalculation {
    type Error = String;

    fn try_from(result: RewardsCalculatorResults) -> Result<Self, Self::Error> {
        let RewardsCalculatorResults {
            base_rewards_xdr_permyriad_by_category,
            results_by_node,
            extrapolated_fr,
            rewards_total_xdr_permyriad,
        } = result;

        let mut daily_subnets_fr = BTreeMap::new();
        for (_, metrics) in results_by_node.iter() {
            for metric in metrics.daily_metrics.iter() {
                daily_subnets_fr
                    .entry((metric.subnet_assigned, metric.ts))
                    .or_insert(metric.subnet_assigned_fr);
            }
        }
        let daily_subnets_fr = daily_subnets_fr
            .into_iter()
            .map(|((subnet_id, ts), fr)| {
                let utc_day = timestamp_to_utc_date(ts.get());
                let fr = fr.round_dp(4).to_f64().err_f64()?;

                Ok((subnet_id, DailySubnetFailureRate { utc_day, fr }))
            })
            .collect::<Result<Vec<_>, String>>()?
            .into_iter()
            .into_group_map()
            .into_iter()
            .collect();

        let rewards_by_category: Vec<_> = base_rewards_xdr_permyriad_by_category
            .into_iter()
            .map(|(category, rewards)| {
                Ok(BaseRewardsByCategory {
                    node_type: category.node_type.to_string(),
                    region: category.region.to_string(),
                    base_rewards: (rewards / dec!(10000)).round_dp(4).to_f64().err_f64()?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?
            .into_iter()
            .collect();

        let extrapolated_fr = extrapolated_fr.round_dp(4).to_f64().err_f64()?;

        let rewards_total_xdr = (rewards_total_xdr_permyriad / dec!(10000)).to_u64().err_u64()?;

        let results_by_node = results_by_node
            .into_iter()
            .map(|(node_id, node_results)| {
                let node_type = node_results.node_type.clone();
                let region = node_results.region.clone();
                let rewards_reduction = node_results.rewards_reduction.round_dp(4).to_f64().err_f64()?;
                let performance_multiplier = node_results.performance_multiplier.round_dp(4).to_f64().err_f64()?;
                let adjusted_rewards = (node_results.adjusted_rewards_xdr_permyriad / dec!(10000))
                    .round_dp(4)
                    .to_f64()
                    .err_f64()?;
                let average_fr = node_results
                    .avg_relative_fr
                    .map_or(extrapolated_fr, |fr| fr.round_dp(4).to_f64().unwrap());

                let daily_node_results: Vec<_> = node_results
                    .daily_metrics
                    .into_iter()
                    .map(|daily_metrics| {
                        Ok(DailyNodeResults {
                            utc_day: timestamp_to_utc_date(daily_metrics.ts.get()),
                            subnet_assigned: daily_metrics.subnet_assigned,
                            blocks_proposed: daily_metrics.num_blocks_proposed,
                            blocks_failed: daily_metrics.num_blocks_failed,
                            original_failure_rate: daily_metrics.original_fr.round_dp(4).to_f64().err_f64()?,
                            relative_failure_rate: daily_metrics.relative_fr.round_dp(4).to_f64().err_f64()?,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                let daily_node_results = if daily_node_results.is_empty() { None } else { Some(daily_node_results) };

                let base_rewards = rewards_by_category
                    .iter()
                    .find(|category| category.node_type == node_results.node_type && category.region == node_results.region)
                    .ok_or(format!(
                        "Base rewards not found for node type: {} and region: {}",
                        node_results.node_type.clone(),
                        node_results.region.clone()
                    ))?
                    .base_rewards;

                Ok((
                    node_id,
                    NodeResults {
                        node_type,
                        region,
                        daily_node_results,
                        average_fr,
                        rewards_reduction,
                        performance_multiplier,
                        base_rewards,
                        adjusted_rewards,
                    },
                ))
            })
            .collect::<Result<BTreeMap<_, _>, String>>()?;

        Ok(Self {
            daily_subnets_fr,
            extrapolated_fr,
            results_by_node,
            rewards_by_category,
            rewards_total_xdr,
        })
    }
}

fn timestamp_to_utc_date(ts: TimestampNanos) -> String {
    DateTime::from_timestamp(ts as i64 / 1_000_000_000, 0)
        .unwrap()
        .naive_utc()
        .format("%d-%m-%Y")
        .to_string()
}

trait OptionExt<T> {
    fn err_f64(self) -> Result<T, String>;
    fn err_u64(self) -> Result<T, String>;
}

impl<T> OptionExt<T> for Option<T> {
    fn err_f64(self) -> Result<T, String> {
        self.ok_or_else(|| "Failed to convert Decimal to f64".to_string())
    }

    fn err_u64(self) -> Result<T, String> {
        self.ok_or_else(|| "Failed to convert Decimal to u64".to_string())
    }
}
