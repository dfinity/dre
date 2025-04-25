use chrono::DateTime;
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use rewards_calculation::rewards_calculator_results;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

#[derive(candid::CandidType, candid::Deserialize)]
pub struct RewardPeriodArgs {
    pub start_ts: u64,
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

#[derive(candid::CandidType, candid::Deserialize)]
pub struct DayUTC(String);
impl From<rewards_calculator_results::DayUTC> for DayUTC {
    fn from(value: rewards_calculator_results::DayUTC) -> Self {
        let dd_mm_yyyy = DateTime::from_timestamp(value.ts_at_day_end() as i64 / 1_000_000_000, 0)
            .unwrap()
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

#[derive(candid::CandidType, candid::Deserialize)]
/// see [`rewards_calculator_results::NodeResults`]
pub struct NodeResults {
    pub node_type: String,
    pub region: String,
    pub dc_id: String,
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
                let region = node_results.region.to_string();
                let node_type = node_results.node_type.to_string();
                let dc_id = node_results.dc_id.to_string();
                let avg_relative_fr = node_results.avg_relative_fr.map(|fr| fr.try_into()).transpose()?;

                let daily_node_results: Vec<_> = node_results
                    .daily_metrics
                    .into_iter()
                    .map(|daily_metrics| {
                        Ok(NodeMetricsDaily {
                            day: daily_metrics.day.into(),
                            subnet_assigned: daily_metrics.subnet_assigned,
                            subnet_assigned_fr: daily_metrics.subnet_assigned_fr.try_into()?,
                            num_blocks_proposed: daily_metrics.num_blocks_proposed,
                            num_blocks_failed: daily_metrics.num_blocks_failed,
                            original_fr: daily_metrics.original_fr.try_into()?,
                            relative_fr: daily_metrics.relative_fr.try_into()?,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?;

                Ok((
                    node_id,
                    NodeResults {
                        node_type,
                        region,
                        dc_id,
                        daily_metrics: daily_node_results,
                        avg_relative_fr,
                        rewardable_days: node_results.rewardable_days as u64,
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
