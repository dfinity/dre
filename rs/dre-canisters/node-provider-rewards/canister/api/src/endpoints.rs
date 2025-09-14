use chrono::DateTime;
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use rewards_calculation::rewards_calculator_results;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

// FIXME: these fields need to be documented!  Are they inclusive or exclusive ranges?  How does this work?
#[derive(candid::CandidType, candid::Deserialize, Clone)]
pub struct RewardPeriodArgs {
    /// Start of the reward distribution period, as a Unix timestamp in nanoseconds.
    /// This timestamp is covers the entire correspondent UTC day and is inclusive.
    pub start_ts: u64,

    /// End of the reward distribution period, as a Unix timestamp in nanoseconds.
    /// This timestamp is covers the entire correspondent UTC day and is inclusive.
    pub end_ts: u64,
}

#[derive(candid::CandidType, candid::Deserialize)]
pub struct NodeProviderRewardsCalculationArgs {
    pub provider_id: PrincipalId,
    pub reward_period: RewardPeriodArgs,
}

fn decimal_to_f64(value: Decimal) -> Result<f64, String> {
    value.round_dp(4).to_f64().ok_or_else(|| "Failed to convert Decimal to f64".to_string())
}

#[derive(candid::CandidType, candid::Deserialize)]
pub struct XDRPermyriad(f64);
impl TryFrom<rewards_calculator_results::XDRPermyriad> for XDRPermyriad {
    type Error = String;

    fn try_from(value: rewards_calculator_results::XDRPermyriad) -> Result<Self, Self::Error> {
        Ok(Self(decimal_to_f64(value.get())?))
    }
}

#[derive(candid::CandidType, candid::Deserialize)]
pub struct Percent(f64);
impl TryFrom<rewards_calculator_results::Percent> for Percent {
    type Error = String;

    fn try_from(value: rewards_calculator_results::Percent) -> Result<Self, Self::Error> {
        Ok(Self(decimal_to_f64(value.get())?))
    }
}

#[derive(candid::CandidType, candid::Deserialize, Ord, PartialOrd, Eq, PartialEq)]
pub struct DayUTC(String);
impl From<rewards_calculator_results::DayUTC> for DayUTC {
    fn from(value: rewards_calculator_results::DayUTC) -> Self {
        let dd_mm_yyyy = DateTime::from_timestamp_nanos(value.unix_ts_at_day_end() as i64)
            .naive_utc()
            .format("%d-%m-%Y")
            .to_string();

        Self(dd_mm_yyyy)
    }
}

#[derive(candid::CandidType, candid::Deserialize)]
pub struct NodeProvidersRewards {
    pub rewards_per_provider: BTreeMap<PrincipalId, XDRPermyriad>,
}

impl TryFrom<BTreeMap<PrincipalId, rewards_calculator_results::XDRPermyriad>> for NodeProvidersRewards {
    type Error = String;

    fn try_from(rewards_per_provider: BTreeMap<PrincipalId, rewards_calculator_results::XDRPermyriad>) -> Result<Self, Self::Error> {
        let rewards_xdr_permyriad_per_provider = rewards_per_provider
            .into_iter()
            .map(|(k, v)| Ok((k, v.try_into()?)))
            .collect::<Result<BTreeMap<PrincipalId, XDRPermyriad>, String>>()?;
        Ok(Self {
            rewards_per_provider: rewards_xdr_permyriad_per_provider,
        })
    }
}

#[derive(candid::CandidType, candid::Deserialize)]
/// see [`rewards_calculator_results::NodeMetricsDaily`]
pub struct NodeMetricsDaily {
    pub day: DayUTC,
    pub subnet_assigned: SubnetId,
    pub subnet_assigned_fr: Percent,
    pub num_blocks_proposed: u64,
    pub num_blocks_failed: u64,
    pub original_fr: Percent,
    pub relative_fr: Percent,
}

impl TryFrom<rewards_calculator_results::NodeMetricsDaily> for NodeMetricsDaily {
    type Error = String;

    fn try_from(value: rewards_calculator_results::NodeMetricsDaily) -> Result<Self, Self::Error> {
        Ok(Self {
            day: value.day.into(),
            subnet_assigned: value.subnet_assigned,
            subnet_assigned_fr: value.subnet_assigned_fr.try_into()?,
            num_blocks_proposed: value.num_blocks_proposed,
            num_blocks_failed: value.num_blocks_failed,
            original_fr: value.original_fr.try_into()?,
            relative_fr: value.relative_fr.try_into()?,
        })
    }
}

#[derive(candid::CandidType, candid::Deserialize)]
/// see [`rewards_calculator_results::NodeResults`]
pub struct NodeResults {
    pub node_type: String,
    pub region: String,
    pub dc_id: String,
    pub rewardable_from: DayUTC,
    pub rewardable_to: DayUTC,
    pub rewardable_days: u64,
    pub daily_metrics: Vec<NodeMetricsDaily>,
    pub avg_relative_fr: Option<Percent>,
    pub avg_extrapolated_fr: Percent,
    pub rewards_reduction: Percent,
    pub performance_multiplier: Percent,
    pub base_rewards_per_month: XDRPermyriad,
    pub base_rewards: XDRPermyriad,
    pub adjusted_rewards: XDRPermyriad,
}

#[derive(candid::CandidType, candid::Deserialize)]
/// see [`rewards_calculator_results::RewardsCalculatorResults`]
pub struct RewardsCalculatorResults {
    pub results_by_node: BTreeMap<NodeId, NodeResults>,
    pub extrapolated_fr: Percent,
    pub rewards_total: XDRPermyriad,
}

impl TryFrom<rewards_calculator_results::RewardsCalculatorResults> for RewardsCalculatorResults {
    type Error = String;

    fn try_from(value: rewards_calculator_results::RewardsCalculatorResults) -> Result<Self, Self::Error> {
        let results_by_node = value
            .results_by_node
            .into_iter()
            .map(|(node_id, node_results)| {
                let region = node_results.region.0;
                let node_type = node_results.node_reward_type.as_str_name().to_string();
                let dc_id = node_results.dc_id.to_string();
                let avg_relative_fr = node_results.avg_relative_fr.map(|fr| fr.try_into()).transpose()?;

                let daily_metrics: Vec<_> = node_results
                    .daily_metrics
                    .into_iter()
                    .map(|daily_metrics| daily_metrics.try_into())
                    .collect::<Result<Vec<_>, String>>()?;

                Ok((
                    node_id,
                    NodeResults {
                        node_type,
                        region,
                        dc_id,
                        daily_metrics,
                        avg_relative_fr,
                        rewardable_from: (*node_results.rewardable_days.first().unwrap()).into(),
                        rewardable_to: (*node_results.rewardable_days.last().unwrap()).into(),
                        rewardable_days: node_results.rewardable_days.len() as u64,
                        avg_extrapolated_fr: node_results.avg_extrapolated_fr.try_into()?,
                        rewards_reduction: node_results.rewards_reduction.try_into()?,
                        performance_multiplier: node_results.performance_multiplier.try_into()?,
                        base_rewards_per_month: node_results.base_rewards_per_month.try_into()?,
                        base_rewards: node_results.base_rewards.try_into()?,
                        adjusted_rewards: node_results.adjusted_rewards.try_into()?,
                    },
                ))
            })
            .collect::<Result<BTreeMap<_, _>, String>>()?;

        Ok(Self {
            results_by_node,
            extrapolated_fr: value.extrapolated_fr.try_into()?,
            rewards_total: value.rewards_total.try_into()?,
        })
    }
}

#[derive(candid::CandidType, candid::Deserialize)]
pub enum NodeStatus {
    Assigned { node_metrics: NodeMetricsDaily },
    Unassigned { extrapolated_fr: Percent },
}

#[derive(candid::CandidType, candid::Deserialize)]
pub struct DailyResults {
    pub node_status: NodeStatus,
    pub rewards_reduction: Percent,
    pub performance_multiplier: Percent,
    pub base_rewards: XDRPermyriad,
    pub adjusted_rewards: XDRPermyriad,
}

#[derive(candid::CandidType, candid::Deserialize)]
pub struct NodeResultsV1 {
    pub node_type: String,
    pub region: String,
    pub dc_id: String,
    pub daily_results: BTreeMap<DayUTC, DailyResults>,
}

#[derive(candid::CandidType, candid::Deserialize)]
pub struct RewardsCalculatorResultsV1 {
    pub results_by_node: BTreeMap<NodeId, NodeResultsV1>,
    pub rewards_total: XDRPermyriad,
}

impl TryFrom<rewards_calculator_results::RewardsCalculatorResults> for RewardsCalculatorResultsV1 {
    type Error = String;

    fn try_from(value: rewards_calculator_results::RewardsCalculatorResults) -> Result<Self, Self::Error> {
        let results_by_node = value
            .results_by_node
            .into_iter()
            .map(|(node_id, mut node_results)| {
                let region = node_results.region.0.clone();
                let node_type = node_results.node_reward_type.as_str_name().to_string();
                let dc_id = node_results.dc_id.to_string();
                let daily_results: BTreeMap<DayUTC, DailyResults> = node_results
                    .rewardable_days
                    .into_iter()
                    .map(|day| {
                        let node_status = match node_results.daily_metrics_v1.remove(&day) {
                            Some(daily_metrics) => NodeStatus::Assigned {
                                node_metrics: NodeMetricsDaily::try_from(daily_metrics)?,
                            },
                            None => NodeStatus::Unassigned {
                                extrapolated_fr: value
                                    .extrapolated_fr_v1
                                    .get(&day)
                                    .expect("Extrapolated failure rate must exist for each rewardable day")
                                    .clone()
                                    .try_into()?,
                            },
                        };
                        let performance_multiplier = node_results
                            .performance_multiplier_v1
                            .remove(&day)
                            .expect("Performance multiplier must exist for each rewardable day")
                            .try_into()?;
                        let rewards_reduction = node_results
                            .rewards_reduction_v1
                            .remove(&day)
                            .expect("Performance multiplier must exist for each rewardable day")
                            .try_into()?;
                        let base_rewards = value
                            .base_rewards_by_category_v1
                            .get(&(day, node_results.region.clone(), node_results.node_reward_type))
                            .expect("Base rewards must exist for each rewardable day")
                            .clone()
                            .try_into()?;
                        let adjusted_rewards = node_results
                            .adjusted_rewards_v1
                            .remove(&day)
                            .expect("Adjusted rewards must exist for each rewardable day")
                            .try_into()?;

                        Ok((
                            day.into(),
                            DailyResults {
                                node_status,
                                rewards_reduction,
                                performance_multiplier,
                                base_rewards,
                                adjusted_rewards,
                            },
                        ))
                    })
                    .collect::<Result<BTreeMap<_, _>, String>>()?;

                Ok((
                    node_id,
                    NodeResultsV1 {
                        node_type,
                        region,
                        dc_id,
                        daily_results,
                    },
                ))
            })
            .collect::<Result<BTreeMap<_, _>, String>>()?;

        Ok(Self {
            results_by_node,
            rewards_total: value.rewards_total.try_into()?,
        })
    }
}
