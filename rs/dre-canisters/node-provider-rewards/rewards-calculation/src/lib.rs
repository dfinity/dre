use crate::calculation_results::RewardsCalculatorResults;
use crate::input_builder::{RewardCalculationError, RewardableNode, RewardsCalculatorInput};
use crate::rewards_calculator::{Initialized, RewardsCalculator, RewardsTotalComputed};
use std::marker::PhantomData;

pub mod calculation_results;
pub mod input_builder;
mod rewards_calculator;
pub mod types;

/// Computes rewards for node providers based on their nodes' performance during the specified `reward_period`.
///
/// # Arguments
/// * reward_period - The time frame for which rewards are calculated.
/// * rewards_table - The rewards table containing the reward rates for each node type.
/// * metrics_by_node - Daily node metrics for nodes in `reward_period`. Only nodes in `providers_rewardable_nodes` keys are considered.
/// * rewardable_nodes: Nodes eligible for rewards, as recorded in the registry versions spanning the `reward_period` provided.
pub fn calculate_rewards(
    input: &RewardsCalculatorInput,
    rewardable_nodes: Vec<RewardableNode>,
) -> Result<RewardsCalculatorResults, RewardCalculationError> {
    if rewardable_nodes.is_empty() {
        return Err(RewardCalculationError::EmptyNodes);
    }

    // Performance Multipliers Calculation
    let ctx: RewardsCalculator<Initialized> = RewardsCalculator {
        input,
        rewardable_nodes,
        calculator_results: RewardsCalculatorResults::default(),
        _marker: PhantomData,
    };
    let computed: RewardsCalculator<RewardsTotalComputed> = ctx.next().next().next().next().next().next().next().next().next().next();
    let calculation_results = computed.get_results();

    Ok(calculation_results)
}

#[cfg(test)]
mod tests;
