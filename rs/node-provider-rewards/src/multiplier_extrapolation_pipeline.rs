use crate::logs::{LogEntry, Logger, Operation, OperationCalculator};
use crate::types::{DailyFailureRate, FailureRate, TimestampNanos};
use ic_base_types::{NodeId, SubnetId};
use itertools::Itertools;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use std::fmt;
use tabular::{Row, Table};

const MIN_FAILURE_RATE: Decimal = dec!(0.1);
const MAX_FAILURE_RATE: Decimal = dec!(0.6);
const MAX_REWARDS_REDUCTION: Decimal = dec!(0.8);

pub struct ExecutionContext<'a> {
    logger: Logger,
    calculator: OperationCalculator,
    provider_nodes_failure_rates: HashMap<NodeId, Vec<DailyFailureRate>>,
    subnets_failure_rate: Option<&'a HashMap<(SubnetId, TimestampNanos), Decimal>>,
}

impl<'a> fmt::Display for ExecutionContext<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ExecutionContext {{
                subnets_failure_rate: {:?}
            }}",
            self.subnets_failure_rate
        )
    }
}

impl<'a> ExecutionContext<'a> {
    pub fn new(
        provider_nodes_failure_rates: HashMap<NodeId, Vec<DailyFailureRate>>,
        subnets_failure_rate: Option<&'a HashMap<(SubnetId, TimestampNanos), Decimal>>,
    ) -> Self {
        Self {
            logger: Logger::default(),
            calculator: OperationCalculator,
            provider_nodes_failure_rates,
            subnets_failure_rate,
        }
    }
}

type NodesMultiplier = HashMap<NodeId, Decimal>;
type AvgFailureRatePerNode = HashMap<NodeId, Decimal>;
type FailureRateExtrapolated = Decimal;

#[derive(Debug)]
enum PipelineStep {
    Start,
    ComputeRelativeFailureRates,
    ComputeFailureRateExtrapolated,
    ReplaceUndefinedFailureRates(FailureRateExtrapolated),
    ComputeAverageFailureRatePerNode,
    ComputeRewardsMultiplierPerNode(AvgFailureRatePerNode),
}

enum PipelineOutcome {
    Continue(PipelineStep),
    Finished(NodesMultiplier),
}

impl PipelineStep {
    fn execute(&self, ctx: &mut ExecutionContext) -> PipelineOutcome {
        match (self, ctx) {
            (PipelineStep::Start, _) => PipelineOutcome::Continue(PipelineStep::ComputeRelativeFailureRates),

            // Compute the relative failure rates for each node
            (
                PipelineStep::ComputeRelativeFailureRates,
                ExecutionContext {
                    provider_nodes_failure_rates,
                    subnets_failure_rate,
                    ..
                },
            ) => {
                for daily_rate in provider_nodes_failure_rates.values_mut().flatten() {
                    match daily_rate.failure_rate {
                        FailureRate::Defined { subnet_assigned, value } => match subnets_failure_rate {
                            Some(subnets_failure_rate) => {
                                let subnet_failure_rate = subnets_failure_rate
                                    .get(&(subnet_assigned, daily_rate.ts))
                                    .cloned()
                                    .unwrap_or_else(|| panic!("Subnet failure rate not found for {:?}", (subnet_assigned, daily_rate.ts)));

                                daily_rate.failure_rate = FailureRate::DefinedRelative {
                                    subnet_assigned,
                                    subnet_failure_rate,
                                    original_failure_rate: value,
                                    value: value - subnet_failure_rate,
                                }
                            }
                            None => {
                                daily_rate.failure_rate = FailureRate::DefinedRelative {
                                    subnet_assigned,
                                    subnet_failure_rate: Decimal::ZERO,
                                    original_failure_rate: value,
                                    value,
                                };
                            }
                        },
                        FailureRate::Undefined => continue,
                        _ => {
                            panic!("Expected Defined/Undefined failure rate got: {:?}", daily_rate.failure_rate);
                        }
                    }
                }
                PipelineOutcome::Continue(PipelineStep::ComputeFailureRateExtrapolated)
            }

            // Compute the failure rate extrapolated for unassigned days
            (
                PipelineStep::ComputeFailureRateExtrapolated,
                ExecutionContext {
                    provider_nodes_failure_rates,
                    logger,
                    calculator,
                    ..
                },
            ) => {
                logger.log(LogEntry::ComputeFailureRateExtrapolated);

                let failure_rate_extrapolated: Decimal = if provider_nodes_failure_rates.is_empty() {
                    calculator.run_and_log("No nodes assigned", Operation::Set(dec!(1)), logger)
                } else {
                    let avg_rates: Vec<Decimal> = provider_nodes_failure_rates
                        .iter()
                        .map(|(node_id, failure_rates)| {
                            // Include only the failure rates that are explicitly defined
                            let defined_failure_rates = failure_rates
                                .iter()
                                .filter_map(|daily_failure_rate| match daily_failure_rate.failure_rate {
                                    FailureRate::DefinedRelative { value, .. } => Some(value),
                                    FailureRate::Undefined => None,
                                    _ => panic!("Expected DefinedRelative/Undefined failure rate"),
                                })
                                .collect::<Vec<_>>();

                            calculator.run_and_log(
                                &format!("Average failure rate for node: {}", node_id),
                                Operation::Avg(defined_failure_rates),
                                logger,
                            )
                        })
                        .collect();

                    calculator.run_and_log("Unassigned days failure rate:", Operation::Avg(avg_rates), logger)
                };

                PipelineOutcome::Continue(PipelineStep::ReplaceUndefinedFailureRates(failure_rate_extrapolated))
            }

            // Replace the undefined failure rates with the extrapolated value
            (
                PipelineStep::ReplaceUndefinedFailureRates(failure_rate_extrapolated),
                ExecutionContext {
                    provider_nodes_failure_rates,
                    ..
                },
            ) => {
                for daily_rate in provider_nodes_failure_rates.values_mut().flatten() {
                    match daily_rate.failure_rate {
                        FailureRate::Undefined => {
                            daily_rate.failure_rate = FailureRate::Extrapolated {
                                value: *failure_rate_extrapolated,
                            };
                        }
                        FailureRate::DefinedRelative { .. } => continue,
                        _ => {
                            panic!("Expected Defined/Undefined failure rate got: {:?}", daily_rate.failure_rate);
                        }
                    }
                }

                PipelineOutcome::Continue(PipelineStep::ComputeAverageFailureRatePerNode)
            }

            // Compute the average failure rate for each node
            (
                PipelineStep::ComputeAverageFailureRatePerNode,
                ExecutionContext {
                    provider_nodes_failure_rates,
                    logger,
                    calculator,
                    ..
                },
            ) => {
                let nodes_avg_failure_rate = provider_nodes_failure_rates
                    .iter()
                    .map(|(node_id, daily_failure_rates)| {
                        logger.log(LogEntry::ComputeNodeMultiplier(*node_id));

                        let failure_rates = daily_failure_rates
                            .iter()
                            .map(|rate| match rate.failure_rate {
                                FailureRate::DefinedRelative { value, .. } | FailureRate::Extrapolated { value } => value,
                                _ => panic!("Expected DefinedRelative or Extrapolated failure rate"),
                            })
                            .collect::<Vec<_>>();

                        let avg_failure_rate = calculator.run_and_log("Failure rate average", Operation::Avg(failure_rates), logger);
                        (*node_id, avg_failure_rate)
                    })
                    .collect();
                PipelineOutcome::Continue(PipelineStep::ComputeRewardsMultiplierPerNode(nodes_avg_failure_rate))
            }

            // Compute the rewards multiplier for each node
            (PipelineStep::ComputeRewardsMultiplierPerNode(nodes_avg_failure_rate), ExecutionContext { logger, calculator, .. }) => {
                let multiplier_per_node = nodes_avg_failure_rate
                    .iter()
                    .map(|(node_id, failure_rate)| {
                        let rewards_reduction = {
                            if failure_rate < &MIN_FAILURE_RATE {
                                calculator.run_and_log(
                                    &format!(
                                        "No Reduction applied because {} is less than {} failure rate.\n{}",
                                        failure_rate.round_dp(4),
                                        MIN_FAILURE_RATE,
                                        "Linear Reduction factor"
                                    ),
                                    Operation::Set(dec!(0)),
                                    logger,
                                )
                            } else if failure_rate > &MAX_FAILURE_RATE {
                                calculator.run_and_log(
                                    &format!(
                                        "Max reduction applied because {} is over {} failure rate.\n{}",
                                        failure_rate.round_dp(4),
                                        MAX_FAILURE_RATE,
                                        "Linear Reduction factor"
                                    ),
                                    Operation::Set(dec!(0.8)),
                                    logger,
                                )
                            } else {
                                let rewards_reduction =
                                    (*failure_rate - MIN_FAILURE_RATE) / (MAX_FAILURE_RATE - MIN_FAILURE_RATE) * MAX_REWARDS_REDUCTION;
                                logger.log(LogEntry::RewardsReductionPercent {
                                    failure_rate: *failure_rate,
                                    min_fr: MIN_FAILURE_RATE,
                                    max_fr: MAX_FAILURE_RATE,
                                    max_rr: MAX_REWARDS_REDUCTION,
                                    rewards_reduction,
                                });

                                rewards_reduction
                            }
                        };

                        let multiplier = calculator.run_and_log("Rewards Multiplier", Operation::Subtract(dec!(1), rewards_reduction), logger);
                        (*node_id, multiplier)
                    })
                    .collect();
                PipelineOutcome::Finished(multiplier_per_node)
            }
        }
    }
}

pub struct MultiplierExtrapolationPipeline {
    nodes_failure_rates: HashMap<NodeId, Vec<DailyFailureRate>>,
    subnets_failure_rate: HashMap<(SubnetId, TimestampNanos), Decimal>,
}

impl MultiplierExtrapolationPipeline {
    pub fn new(nodes_failure_rates: HashMap<NodeId, Vec<DailyFailureRate>>) -> Self {
        Self {
            subnets_failure_rate: Self::compute_subnets_failure_rates(&nodes_failure_rates),
            nodes_failure_rates,
        }
    }

    fn compute_subnets_failure_rates(nodes_failure_rates: &HashMap<NodeId, Vec<DailyFailureRate>>) -> HashMap<(SubnetId, TimestampNanos), Decimal> {
        const PERCENTILE: f64 = 0.75;

        nodes_failure_rates
            .values()
            .flatten()
            .filter_map(|metric| match metric.failure_rate {
                FailureRate::Defined { subnet_assigned, value } => Some((subnet_assigned, metric.ts, value)),
                _ => None,
            })
            .chunk_by(|(subnet_assigned, ts, _)| (*subnet_assigned, *ts))
            .into_iter()
            .map(|(key, group)| {
                let subnet_failure_rates: Vec<Decimal> = group.map(|(_, _, failure_rate)| failure_rate).sorted().collect();
                let len = subnet_failure_rates.len();
                if len == 0 {
                    return (key, Decimal::ZERO);
                }

                let idx = ((len as f64) * PERCENTILE).ceil() as usize - 1;
                let failure_rate_percentile = subnet_failure_rates[idx];

                (key, failure_rate_percentile)
            })
            .collect()
    }

    pub fn run(&self, provider_nodes: Vec<NodeId>) -> (NodesMultiplier, Logger) {
        let provider_nodes_failure_rates = provider_nodes
            .iter()
            .map(|node_id| {
                (
                    *node_id,
                    self.nodes_failure_rates.get(node_id).cloned().expect("Node failure rates not found"),
                )
            })
            .collect::<HashMap<_, _>>();

        let mut ctx = ExecutionContext::new(provider_nodes_failure_rates, Some(&self.subnets_failure_rate));
        let mut current_step = PipelineStep::Start;
        loop {
            match current_step.execute(&mut ctx) {
                PipelineOutcome::Continue(next_step) => current_step = next_step,
                PipelineOutcome::Finished(nodes_multiplier) => {
                    let table = generate_table(&ctx.provider_nodes_failure_rates);

                    ctx.logger.log(LogEntry::FinalNodesMultiplier(nodes_multiplier.clone()));
                    ctx.logger.log(LogEntry::FinalExtrapolatedFailureRates(table));

                    return (nodes_multiplier, ctx.logger);
                }
            }
        }
    }
}

fn generate_table(failure_rates: &HashMap<NodeId, Vec<DailyFailureRate>>) -> Table {
    let mut table = Table::new("{:<} {:<} {:<}");

    table.add_row(Row::new().with_cell("Node ID").with_cell("Timestamp").with_cell("Failure Rate"));

    // Data Rows
    for (node_id, rates) in failure_rates {
        for rate in rates {
            table.add_row(
                Row::new()
                    .with_cell(node_id.to_string())
                    .with_cell(rate.ts)
                    .with_cell(format!("{:?}", rate.failure_rate)),
            );
        }
    }

    table.to_owned()
}

#[cfg(test)]
mod tests;
