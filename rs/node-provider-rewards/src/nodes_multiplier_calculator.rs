use crate::failure_rates::{NodeDailyFailureRate, NodeFailureRate, SubnetDailyFailureRate};
use crate::logs::{LogEntry, Logger, Operation, OperationCalculator};
use crate::types::TimestampNanos;
use ic_base_types::{NodeId, SubnetId};
use itertools::Itertools;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use tabular::{Row, Table};

type AvgFailureRatePerNode = HashMap<NodeId, Decimal>;
type FailureRateExtrapolated = Decimal;

const MIN_FAILURE_RATE: Decimal = dec!(0.1);
const MAX_FAILURE_RATE: Decimal = dec!(0.6);
const MAX_REWARDS_REDUCTION: Decimal = dec!(0.8);

pub struct NodesMultiplierOutput {
    pub nodes_multiplier: HashMap<NodeId, Decimal>,
    pub logger: Logger,
}

pub struct NodesMultiplierCalculator<'a> {
    logger: Logger,
    calculator: OperationCalculator,
    nodes_failure_rates: HashMap<NodeId, Vec<NodeDailyFailureRate>>,
    subnets_failure_rate: &'a HashMap<SubnetId, Vec<SubnetDailyFailureRate>>,
}

impl<'a> NodesMultiplierCalculator<'a> {
    pub fn new(
        nodes_failure_rates: HashMap<NodeId, Vec<NodeDailyFailureRate>>,
        subnets_failure_rate: &'a HashMap<SubnetId, Vec<SubnetDailyFailureRate>>,
    ) -> Self {
        Self {
            logger: Logger::default(),
            calculator: OperationCalculator,
            nodes_failure_rates,
            subnets_failure_rate,
        }
    }

    fn discount_subnets_failure_rate(&mut self) {
        for node_daily_failure_rate in self.nodes_failure_rates.values_mut().flatten() {
            match node_daily_failure_rate.value {
                NodeFailureRate::Defined { subnet_assigned, value } => {
                    let subnet_failure_rate = self
                        .subnets_failure_rate
                        .get(&subnet_assigned)
                        .and_then(|rates| {
                            rates
                                .iter()
                                .find(|subnet_daily_failure_rate| subnet_daily_failure_rate.ts == node_daily_failure_rate.ts)
                        })
                        .cloned()
                        .expect("Subnet failure rate not found")
                        .value;

                    node_daily_failure_rate.value = NodeFailureRate::DefinedRelative {
                        subnet_assigned,
                        subnet_failure_rate,
                        original_failure_rate: value,
                        value: value - subnet_failure_rate,
                    };
                }
                NodeFailureRate::Undefined => continue,
                _ => {
                    panic!("Expected Defined/Undefined failure rate got: {:?}", node_daily_failure_rate);
                }
            }
        }
    }

    fn compute_failure_rate_extrapolated(&mut self) -> Decimal {
        self.logger.log(LogEntry::ComputeFailureRateExtrapolated);

        if self.nodes_failure_rates.is_empty() {
            return self
                .calculator
                .run_and_log("No nodes assigned", Operation::Set(dec!(1)), &mut self.logger);
        };
        let avg_rates: Vec<Decimal> = self
            .nodes_failure_rates
            .iter()
            .map(|(node_id, failure_rates)| {
                // Include only the failure rates that are explicitly defined.
                let defined_failure_rates: Vec<Decimal> = failure_rates
                    .iter()
                    .filter_map(|daily_failure_rate| match daily_failure_rate.value {
                        NodeFailureRate::DefinedRelative { value, .. } => Some(value),
                        NodeFailureRate::Undefined => None,
                        _ => panic!("Expected DefinedRelative/Undefined failure rate got: {:?}", daily_failure_rate),
                    })
                    .collect();

                self.calculator.run_and_log(
                    &format!("Average failure rate for node: {}", node_id),
                    Operation::Avg(defined_failure_rates),
                    &mut self.logger,
                )
            })
            .collect();

        self.calculator
            .run_and_log("Unassigned days failure rate:", Operation::Avg(avg_rates), &mut self.logger)
    }

    fn replace_undefined_failure_rates(&mut self, failure_rate_extrapolated: &Decimal) {
        for daily_rate in self.nodes_failure_rates.values_mut().flatten() {
            match daily_rate.value {
                NodeFailureRate::Undefined => {
                    daily_rate.value = NodeFailureRate::Extrapolated {
                        value: *failure_rate_extrapolated,
                    };
                }
                NodeFailureRate::DefinedRelative { .. } => continue,
                _ => panic!("Expected DefinedRelative/Undefined failure rate got: {:?}", daily_rate),
            }
        }
    }

    fn compute_average_failure_rate_per_node(&mut self) -> HashMap<NodeId, Decimal> {
        self.nodes_failure_rates
            .iter()
            .map(|(node_id, daily_failure_rates)| {
                self.logger.log(LogEntry::ComputeNodeMultiplier(*node_id));

                let failure_rates: Vec<Decimal> = daily_failure_rates
                    .iter()
                    .map(|rate| match rate.value {
                        NodeFailureRate::DefinedRelative { value, .. } | NodeFailureRate::Extrapolated { value } => value,
                        _ => panic!("Expected DefinedRelative or Extrapolated failure rate"),
                    })
                    .collect();

                let avg_failure_rate = self
                    .calculator
                    .run_and_log("Failure rate average", Operation::Avg(failure_rates), &mut self.logger);
                (*node_id, avg_failure_rate)
            })
            .collect()
    }

    fn compute_rewards_multiplier_per_node(&mut self, nodes_avg_failure_rate: HashMap<NodeId, Decimal>) -> HashMap<NodeId, Decimal> {
        nodes_avg_failure_rate
            .iter()
            .map(|(node_id, failure_rate)| {
                let rewards_reduction = if failure_rate < &MIN_FAILURE_RATE {
                    self.calculator.run_and_log(
                        &format!(
                            "No Reduction applied because {} is less than {} failure rate.\n{}",
                            failure_rate.round_dp(4),
                            MIN_FAILURE_RATE,
                            "Linear Reduction factor"
                        ),
                        Operation::Set(dec!(0)),
                        &mut self.logger,
                    )
                } else if failure_rate > &MAX_FAILURE_RATE {
                    self.calculator.run_and_log(
                        &format!(
                            "Max reduction applied because {} is over {} failure rate.\n{}",
                            failure_rate.round_dp(4),
                            MAX_FAILURE_RATE,
                            "Linear Reduction factor"
                        ),
                        Operation::Set(dec!(0.8)),
                        &mut self.logger,
                    )
                } else {
                    let rewards_reduction = (*failure_rate - MIN_FAILURE_RATE) / (MAX_FAILURE_RATE - MIN_FAILURE_RATE) * MAX_REWARDS_REDUCTION;
                    self.logger.log(LogEntry::RewardsReductionPercent {
                        failure_rate: *failure_rate,
                        min_fr: MIN_FAILURE_RATE,
                        max_fr: MAX_FAILURE_RATE,
                        max_rr: MAX_REWARDS_REDUCTION,
                        rewards_reduction,
                    });
                    rewards_reduction
                };

                let multiplier = self
                    .calculator
                    .run_and_log("Rewards Multiplier", Operation::Subtract(dec!(1), rewards_reduction), &mut self.logger);
                (*node_id, multiplier)
            })
            .collect()
    }

    pub fn run(mut self) -> NodesMultiplierOutput {
        // Step 1: Discount subnet performance.
        self.discount_subnets_failure_rate();

        // Step 2: Compute the failure rate extrapolated for unassigned days.
        let failure_rate_extrapolated = self.compute_failure_rate_extrapolated();

        // Step 3: Replace undefined failure rates with the extrapolated value.
        self.replace_undefined_failure_rates(&failure_rate_extrapolated);

        // Step 4: Compute the average failure rate for each node.
        let nodes_avg_failure_rate = self.compute_average_failure_rate_per_node();

        // Step 5: Compute the rewards multiplier for each node.
        let nodes_multiplier = self.compute_rewards_multiplier_per_node(nodes_avg_failure_rate);

        NodesMultiplierOutput {
            nodes_multiplier,
            logger: self.logger,
        }
    }
}

fn generate_table(failure_rates: &HashMap<NodeId, Vec<NodeDailyFailureRate>>) -> Table {
    let mut table = Table::new("{:<} {:<} {:<}");

    table.add_row(Row::new().with_cell("Node ID").with_cell("Timestamp").with_cell("Failure Rate"));

    // Data Rows
    for (node_id, rates) in failure_rates {
        for rate in rates {
            table.add_row(
                Row::new()
                    .with_cell(node_id.to_string())
                    .with_cell(rate.ts)
                    .with_cell(format!("{:?}", rate.value)),
            );
        }
    }

    table.to_owned()
}

#[cfg(test)]
mod tests;
