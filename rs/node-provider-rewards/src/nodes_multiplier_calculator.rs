use crate::logs::{LogEntry, Logger, Operation};
use crate::metrics::{DailyNodeFailureRate, DailySubnetFailureRate, NodeFailureRate};
use crate::reward_period::TimestampNanos;
use crate::tabled_types::generate_table_summary;
use function_name::named;
use ic_base_types::{NodeId, SubnetId};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::BTreeMap;

const MIN_FAILURE_RATE: Decimal = dec!(0.1);
const MAX_FAILURE_RATE: Decimal = dec!(0.6);

const MIN_REWARDS_REDUCTION: Decimal = dec!(0);
const MAX_REWARDS_REDUCTION: Decimal = dec!(0.8);

/// The result of the rewards multiplier calculation.
pub struct RewardsMultipliers {
    /// The computed rewards multipliers per node.
    pub node_rewards_multipliers: BTreeMap<NodeId, Decimal>,
    /// The logger capturing all the computation steps.
    pub logger: Logger,
}

pub struct RewardsCalculator {
    logger: RefCell<Logger>,
    node_failure_data: RefCell<BTreeMap<NodeId, Vec<DailyNodeFailureRate>>>,
    subnet_failure_data: Option<BTreeMap<SubnetId, Vec<DailySubnetFailureRate>>>,
}

impl RewardsCalculator {
    pub fn new() -> Self {
        Self {
            subnet_failure_data: None,
            logger: RefCell::new(Logger::default()),
            node_failure_data: RefCell::new(BTreeMap::new()),
        }
    }

    /// Provides subnet failure data for discount calculations.
    pub fn with_subnet_failure_data(mut self, data: BTreeMap<SubnetId, Vec<DailySubnetFailureRate>>) -> Self {
        self.subnet_failure_data = Some(data);
        self
    }

    fn logger_mut(&self) -> RefMut<Logger> {
        self.logger.borrow_mut()
    }

    fn node_failure_data_mut(&self) -> RefMut<BTreeMap<NodeId, Vec<DailyNodeFailureRate>>> {
        self.node_failure_data.borrow_mut()
    }

    fn node_failure_data(&self) -> Ref<BTreeMap<NodeId, Vec<DailyNodeFailureRate>>> {
        self.node_failure_data.borrow()
    }

    fn execute_and_log(&self, description: &str, operation: Operation) -> Decimal {
        let result = operation.execute();
        self.logger_mut().log(LogEntry::Execute {
            reason: format!("\t{}", description),
            operation,
            result,
        });
        result
    }

    fn lookup_subnet_failure_rate(
        &self,
        subnet: SubnetId,
        timestamp: TimestampNanos,
        subnet_data: &BTreeMap<SubnetId, Vec<DailySubnetFailureRate>>,
    ) -> Option<Decimal> {
        subnet_data
            .get(&subnet)
            .and_then(|rates| rates.iter().find(|rate| rate.ts == timestamp))
            .map(|rate| rate.value)
    }

    /// Updates node failure rates to be relative to their subnet’s failure rate.
    fn update_relative_failure_rates(&self) {
        for failure_entry in self.node_failure_data_mut().values_mut().flatten() {
            if let NodeFailureRate::Defined { subnet_assigned, value } = failure_entry.value {
                let subnet_failure = match &self.subnet_failure_data {
                    Some(data) => self
                        .lookup_subnet_failure_rate(subnet_assigned, failure_entry.ts, data)
                        .expect("Subnet failure rate not found"),
                    None => Decimal::ZERO,
                };

                let relative_failure = if subnet_failure < value { value - subnet_failure } else { Decimal::ZERO };

                failure_entry.value = NodeFailureRate::DefinedRelative {
                    subnet_assigned,
                    subnet_failure_rate: subnet_failure,
                    original_failure_rate: value,
                    value: relative_failure,
                };
            }
        }
    }

    #[named]
    fn calculate_extrapolated_failure_rate(&self) -> Decimal {
        self.logger_mut().log(LogEntry::NodesMultiplierStep(function_name!()));

        if self.node_failure_data().is_empty() {
            return self.execute_and_log("No nodes assigned", Operation::Set(dec!(1)));
        }

        let node_relative_averages: Vec<Decimal> = self
            .node_failure_data()
            .iter()
            .map(|(node_id, failure_entries)| {
                let relative_rates: Vec<Decimal> = failure_entries
                    .iter()
                    .filter_map(|entry| match entry.value {
                        NodeFailureRate::DefinedRelative { value, .. } => Some(value),
                        _ => None,
                    })
                    .collect();

                self.execute_and_log(&node_id.to_string(), Operation::Avg(relative_rates))
            })
            .collect();

        self.execute_and_log("Extrapolated Failure Rate", Operation::Avg(node_relative_averages))
    }

    fn fill_missing_failure_rates(&self, extrapolated_rate: Decimal) {
        for failure_entry in self.node_failure_data_mut().values_mut().flatten() {
            if matches!(failure_entry.value, NodeFailureRate::Undefined) {
                failure_entry.value = NodeFailureRate::Extrapolated(extrapolated_rate);
            }
        }
    }

    #[named]
    fn calculate_average_failure_rate_by_node(&self) -> BTreeMap<NodeId, Decimal> {
        self.logger_mut().log(LogEntry::NodesMultiplierStep(function_name!()));

        self.node_failure_data()
            .iter()
            .map(|(node_id, failure_entries)| {
                let rates: Vec<Decimal> = failure_entries
                    .iter()
                    .filter_map(|entry| match entry.value {
                        NodeFailureRate::DefinedRelative { value, .. } | NodeFailureRate::Extrapolated(value) => Some(value),
                        _ => None,
                    })
                    .collect();

                let average_rate = self.execute_and_log(&node_id.to_string(), Operation::Avg(rates));
                (*node_id, average_rate)
            })
            .collect()
    }

    #[named]
    fn calculate_node_rewards_multiplier(&self, average_failure_rates: &BTreeMap<NodeId, Decimal>) -> BTreeMap<NodeId, Decimal> {
        self.logger_mut().log(LogEntry::NodesMultiplierStep(function_name!()));

        average_failure_rates
            .iter()
            .map(|(node_id, avg_rate)| {
                let rewards_reduction = if avg_rate < &MIN_FAILURE_RATE {
                    MIN_REWARDS_REDUCTION
                } else if avg_rate > &MAX_FAILURE_RATE {
                    MAX_REWARDS_REDUCTION
                } else {
                    ((*avg_rate - MIN_FAILURE_RATE) / (MAX_FAILURE_RATE - MIN_FAILURE_RATE)) * MAX_REWARDS_REDUCTION
                };
                let multiplier = dec!(1) - rewards_reduction;

                self.logger_mut().log(LogEntry::RewardsMultiplier {
                    node_id: *node_id,
                    failure_rate_in_period: *avg_rate,
                    rewards_reduction,
                    rewards_multiplier: multiplier,
                });
                (*node_id, multiplier)
            })
            .collect()
    }

    /// Computes the rewards multipliers for nodes.
    ///
    /// # Arguments
    /// * `daily_failure_data` - A mapping from node IDs to their respective vectors of daily failure rates.
    pub fn calculate_rewards_multipliers(&self, daily_failure_data: BTreeMap<NodeId, Vec<DailyNodeFailureRate>>) -> RewardsMultipliers {
        self.node_failure_data.replace(daily_failure_data);

        self.update_relative_failure_rates();

        let extrapolated_rate = self.calculate_extrapolated_failure_rate();

        self.fill_missing_failure_rates(extrapolated_rate);

        let node_average_failure_rates = self.calculate_average_failure_rate_by_node();

        let node_rewards_multipliers = self.calculate_node_rewards_multiplier(&node_average_failure_rates);

        for (node_id, failure_entries) in self.node_failure_data.take().into_iter() {
            self.logger_mut().log(LogEntry::Summary(node_id, generate_table_summary(failure_entries)));
        }

        RewardsMultipliers {
            logger: self.logger.take(),
            node_rewards_multipliers,
        }
    }
}

#[cfg(test)]
mod tests;
