use crate::logs::{LogEntry, Logger, Operation};
use crate::metrics::{DailyNodeFailureRate, DailySubnetFailureRate, NodeFailureRate};
use crate::reward_period::TimestampNanos;
use chrono::{DateTime, NaiveDateTime, Utc};
use function_name::named;
use ic_base_types::{NodeId, SubnetId};
use itertools::Itertools;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::BTreeMap;
use tabular::{Row, Table};

const MIN_FAILURE_RATE: Decimal = dec!(0.1);
const MAX_FAILURE_RATE: Decimal = dec!(0.6);

const MIN_REWARDS_REDUCTION: Decimal = dec!(0);
const MAX_REWARDS_REDUCTION: Decimal = dec!(0.8);

pub struct NodesMultiplier {
    pub nodes_multiplier: BTreeMap<NodeId, Decimal>,
    pub logger: Logger,
}

pub struct NodesMultiplierCalculator {
    logger: RefCell<Logger>,
    nodes_failure_rates: RefCell<BTreeMap<NodeId, Vec<DailyNodeFailureRate>>>,
    subnets_failure_rates: Option<BTreeMap<SubnetId, Vec<DailySubnetFailureRate>>>,
}

impl NodesMultiplierCalculator {
    pub fn new() -> Self {
        Self {
            subnets_failure_rates: None,
            logger: RefCell::new(Logger::default()),
            nodes_failure_rates: RefCell::new(BTreeMap::new()),
        }
    }

    pub fn with_subnets_fr_discount(self, subnets_failure_rates: BTreeMap<SubnetId, Vec<DailySubnetFailureRate>>) -> Self {
        Self {
            subnets_failure_rates: Some(subnets_failure_rates),
            ..self
        }
    }

    fn logger(&self) -> RefMut<Logger> {
        self.logger.borrow_mut()
    }

    fn daily_nodes_fr_mut(&self) -> RefMut<BTreeMap<NodeId, Vec<DailyNodeFailureRate>>> {
        self.nodes_failure_rates.borrow_mut()
    }

    fn daily_nodes_fr(&self) -> Ref<BTreeMap<NodeId, Vec<DailyNodeFailureRate>>> {
        self.nodes_failure_rates.borrow()
    }

    fn run_and_log(&self, reason: &str, operation: Operation) -> Decimal {
        let result = operation.execute();
        self.logger().log(LogEntry::Execute {
            reason: format!("----> {}", reason),
            operation,
            result,
        });

        result
    }

    fn extract_defined_rates(&self, daily_rates: &[DailyNodeFailureRate]) -> Vec<Decimal> {
        daily_rates.iter().cloned().filter_map(|rate| rate.value.try_into().ok()).collect()
    }

    fn fetch_subnet_failure_rate(&self, subnet_id: SubnetId, ts: TimestampNanos) -> Option<Decimal> {
        self.subnets_failure_rates.as_ref().and_then(|subnets_failure_rates| {
            subnets_failure_rates
                .get(&subnet_id)
                .and_then(|rates| rates.iter().find(|subnet_daily_failure_rate| subnet_daily_failure_rate.ts == ts))
                .map(|subnet_daily_failure_rate| subnet_daily_failure_rate.value)
        })
    }

    fn discount_subnets_failure_rate(&self) {
        if self.subnets_failure_rates.is_none() {
            return;
        }

        for daily_rate in self.daily_nodes_fr_mut().values_mut().flatten() {
            if let NodeFailureRate::Defined { subnet_assigned, value } = daily_rate.value {
                let subnet_failure_rate = self
                    .fetch_subnet_failure_rate(subnet_assigned, daily_rate.ts)
                    .expect("Subnet failure rate not found");

                daily_rate.value = NodeFailureRate::DefinedRelative {
                    subnet_assigned,
                    subnet_failure_rate,
                    original_failure_rate: value,
                    value: value - subnet_failure_rate,
                };
            }
        }
    }

    #[named]
    fn compute_extrapolated_failure_rate(&self) -> Decimal {
        self.logger().log(LogEntry::NodesMultiplierStep(function_name!()));

        if self.daily_nodes_fr().is_empty() {
            return self.run_and_log("No nodes assigned", Operation::Set(dec!(1)));
        }

        let average_nodes_fr: Vec<Decimal> = self
            .daily_nodes_fr()
            .iter()
            .map(|(node_id, daily_failure_rates)| {
                let defined_only: Vec<Decimal> = self.extract_defined_rates(daily_failure_rates);
                self.run_and_log(
                    &format!("Average failure rate (before unassigned days extrapolation) for node {}", *node_id),
                    Operation::Avg(defined_only),
                )
            })
            .collect();

        self.run_and_log("Extrapolated failure rate", Operation::Avg(average_nodes_fr))
    }

    fn replace_undefined_failure_rates(&self, failure_rate_extrapolated: Decimal) {
        for daily_rate in self.daily_nodes_fr_mut().values_mut().flatten() {
            if matches!(daily_rate.value, NodeFailureRate::Undefined) {
                daily_rate.value = NodeFailureRate::Extrapolated(failure_rate_extrapolated);
            }
        }
    }

    #[named]
    fn compute_average_failure_rate_per_node(&self) -> BTreeMap<NodeId, Decimal> {
        self.logger().log(LogEntry::NodesMultiplierStep(function_name!()));

        self.daily_nodes_fr()
            .iter()
            .map(|(node_id, daily_failure_rates)| {
                let failure_rates: Vec<Decimal> = self.extract_defined_rates(daily_failure_rates);
                let avg_failure_rate = self.run_and_log(
                    &format!("Average failure rate (after unassigned days extrapolation) for node {}", node_id),
                    Operation::Avg(failure_rates),
                );
                (*node_id, avg_failure_rate)
            })
            .collect()
    }

    #[named]
    fn compute_rewards_multiplier_per_node(&self, nodes_avg_failure_rate: BTreeMap<NodeId, Decimal>) -> BTreeMap<NodeId, Decimal> {
        self.logger().log(LogEntry::NodesMultiplierStep(function_name!()));

        nodes_avg_failure_rate
            .iter()
            .map(|(node_id, failure_rate)| {
                let rewards_reduction;

                if failure_rate < &MIN_FAILURE_RATE {
                    rewards_reduction = MIN_REWARDS_REDUCTION;
                } else if failure_rate > &MAX_FAILURE_RATE {
                    rewards_reduction = MAX_REWARDS_REDUCTION;
                } else {
                    rewards_reduction = (*failure_rate - MIN_FAILURE_RATE) / (MAX_FAILURE_RATE - MIN_FAILURE_RATE) * MAX_REWARDS_REDUCTION;
                };
                let rewards_multiplier = dec!(1) - rewards_reduction;

                self.logger().log(LogEntry::RewardsMultiplier {
                    node_id: *node_id,
                    failure_rate: *failure_rate,
                    rewards_reduction,
                    rewards_multiplier,
                });
                (*node_id, rewards_multiplier)
            })
            .collect()
    }

    /// Runs the complete nodes multiplier calculation.
    ///
    /// This public method orchestrates the entire process:
    ///
    /// 1. Stores the provided daily node failure rates.
    /// 2. Applies subnet discounting if subnet failure rates are available.
    /// 3. Computes an extrapolated failure rate and replaces undefined failure rates.
    /// 4. Computes the average failure rate for each node.
    /// 5. Determines the rewards multiplier for each node based on their average failure rate.
    /// 6. Logs the final table of failure rates.
    ///
    /// # Arguments
    ///
    /// * `daily_nodes_fr` - A mapping from node IDs to their respective vectors of daily failure rates.
    ///
    /// # Returns
    ///
    /// A `NodesMultiplier` struct containing:
    /// - The computed rewards multipliers per node.
    /// - The logger capturing all the computation steps.
    pub fn run(&self, daily_nodes_fr: BTreeMap<NodeId, Vec<DailyNodeFailureRate>>) -> NodesMultiplier {
        self.nodes_failure_rates.replace(daily_nodes_fr);

        self.discount_subnets_failure_rate();

        let failure_rate_extrapolated = self.compute_extrapolated_failure_rate();

        self.replace_undefined_failure_rates(failure_rate_extrapolated);

        let nodes_avg_failure_rate = self.compute_average_failure_rate_per_node();

        let nodes_multiplier = self.compute_rewards_multiplier_per_node(nodes_avg_failure_rate);

        self.logger().log(LogEntry::Summary(generate_table(&self.daily_nodes_fr())));

        NodesMultiplier {
            logger: self.logger.replace(Logger::default()),
            nodes_multiplier,
        }
    }
}

fn generate_table(failure_rates: &BTreeMap<NodeId, Vec<DailyNodeFailureRate>>) -> Table {
    let mut table = Table::new("{:<} {:<} {:<}");

    table.add_row(Row::new().with_cell("Node ID").with_cell("Day (UTC)").with_cell("Failure Rate"));

    // Data Rows
    for (node_id, rates) in failure_rates {
        for rate in rates {
            table.add_row(
                Row::new()
                    .with_cell(node_id.to_string())
                    .with_cell(
                        DateTime::from_timestamp(rate.ts as i64 / 1_000_000_000, 0)
                            .unwrap()
                            .naive_utc()
                            .to_string(),
                    )
                    .with_cell(format!("{:?}", rate.value)),
            );
        }
    }

    table.to_owned()
}

#[cfg(test)]
mod tests;
