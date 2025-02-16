use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::rc::Rc;
use ic_base_types::{NodeId, SubnetId};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tabular::{Row, Table};
use crate::logs::{LogEntry, Logger, Operation, OperationCalculator};
use crate::types::{DailyFailureRate, FailureRate, TimestampNanos};

const MIN_FAILURE_RATE: Decimal = dec!(0.1);
const MAX_FAILURE_RATE: Decimal = dec!(0.6);
const MAX_REWARDS_REDUCTION: Decimal = dec!(0.8);
const RF: &str = "Linear Reduction factor";

pub type NodesMultiplier = HashMap<NodeId, Decimal>;

pub struct MultiplierExtrapolationPipeline<'a> {
    logger: Rc<Logger>,
    calculator: OperationCalculator,
    nodes_failure_rates: Rc<RefCell<HashMap<NodeId, Vec<DailyFailureRate>>>>,
    subnets_failure_rate: &'a HashMap<(SubnetId, TimestampNanos), Decimal>,
}

impl<'a> MultiplierExtrapolationPipeline<'a> {
    pub fn new(
        nodes_failure_rates: HashMap<NodeId, Vec<DailyFailureRate>>,
        subnets_failure_rate: &'a HashMap<(SubnetId, TimestampNanos), Decimal>,
    ) -> Self {
        let logger = Rc::new(Logger::default());
        let calculator = OperationCalculator::new()
            .with_logger(Rc::clone(&logger));
        let nodes_failure_rates = Rc::new(RefCell::new(nodes_failure_rates));

        Self {
            logger,
            calculator,
            nodes_failure_rates,
            subnets_failure_rate,
        }
    }

    fn nodes_failure_rates(&self) -> Ref<HashMap<NodeId, Vec<DailyFailureRate>>> {
        self.nodes_failure_rates.borrow()
    }

    fn nodes_failure_rates_mut(&self) -> RefMut<HashMap<NodeId, Vec<DailyFailureRate>>> {
        self.nodes_failure_rates.borrow_mut()
    }

    /// Computes the daily relative node failure rates in the period
    fn step_1_calculate_relative_failure_rates(&self) {
        for daily_rate in self.nodes_failure_rates_mut().values_mut().flatten() {
            match daily_rate.value {
                FailureRate::Defined { subnet_assigned, value } => {
                    let systematic_failure_rate = self.subnets_failure_rate
                        .get(&(subnet_assigned, daily_rate.ts))
                        .cloned()
                        .expect("Systematic failure rate not found");

                    let relative_failure_rate = if systematic_failure_rate < value {
                        Decimal::ZERO
                    } else {
                        systematic_failure_rate - value
                    };

                    daily_rate.value = FailureRate::DefinedRelative {
                        subnet_assigned,
                        systematic_failure_rate,
                        original_failure_rate: value,
                        value: relative_failure_rate,
                    };
                },
                _ => continue,
            }
        }
    }

    fn step_2_extrapolate_failure_rate(&self) -> Decimal {
        self.logger.log(LogEntry::ComputeFailureRateExtrapolation);
        let calculator = self.calculator();
        let node_failure_rates = self.nodes_failure_rates();

        if node_failure_rates.is_empty() {
            return calculator.run("No nodes assigned", Operation::Set(dec!(1)));
        }

        let avg_rates: Vec<Decimal> = node_failure_rates
            .values()
            .map(|failure_rates| {
                // Include only the failure rates that are explicitly defined
                failure_rates.iter()
                    .filter_map(|rate| {
                        match rate.value {
                            FailureRate::DefinedRelative { value, .. } => Some(value),
                            _ => None,
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .map(|filtered_rates| calculator.run("Average failure rate", Operation::Avg(filtered_rates)))
            .collect();

        calculator.run("Unassigned days failure rate:", Operation::Avg(avg_rates))
    }

    /// Computes the daily relative node failure rates in the period
    fn step_3_replace_undefined_failure_rates(&self, extrapolated_failure_rate: Decimal) {
        for daily_rate in self.nodes_failure_rates_mut().values_mut().flatten() {
            match daily_rate.value {
                FailureRate::Undefined => {
                    daily_rate.value = FailureRate::Extrapolated {
                        value: extrapolated_failure_rate,
                    };
                },
                _ => continue,
            }
        }
    }

    fn step_4_compute_multiplier_per_node(&self) -> HashMap<NodeId, Decimal> {
        self
            .nodes_failure_rates_mut()
            .iter()
            .map(|(node_id, rates)| {
                self.logger.log(LogEntry::ComputeNodeMultiplier(*node_id));

                let failure_rates = rates.iter().map(|rate| {
                    match rate.value {
                        FailureRate::DefinedRelative { value, .. } | FailureRate::Extrapolated { value } => value,
                        _ => panic!("Expected DefinedRelative or Extrapolated failure rate"),
                    }
                }).collect::<Vec<_>>();

                let avg_rate = self.calculator().run("Failure rate average", Operation::Avg(failure_rates));
                let reduction = self.rewards_reduction_percent(&avg_rate);
                let multiplier =
                    self.calculator().run("Reward Multiplier", Operation::Subtract(dec!(1), reduction));

                (*node_id, multiplier)
            })
            .collect()
    }

    /// Calculates the rewards reduction based on the failure rate.
    ///
    /// if `failure_rate` is:
    /// - Below the `MIN_FAILURE_RATE`, no reduction in rewards applied.
    /// - Above the `MAX_FAILURE_RATE`, maximum reduction in rewards applied.
    /// - Within the defined range (`MIN_FAILURE_RATE` to `MAX_FAILURE_RATE`),
    ///   the function calculates the reduction from the linear reduction function.
    fn rewards_reduction_percent(&self, failure_rate: &Decimal) -> Decimal {
        if failure_rate < &MIN_FAILURE_RATE {
            self.calculator.run(
                &format!(
                    "No Reduction applied because {} is less than {} failure rate.\n{}",
                    failure_rate.round_dp(4),
                    MIN_FAILURE_RATE,
                    RF
                ),
                Operation::Set(dec!(0)),
            )
        } else if failure_rate > &MAX_FAILURE_RATE {
            self.calculator.run(
                &format!(
                    "Max reduction applied because {} is over {} failure rate.\n{}",
                    failure_rate.round_dp(4),
                    MAX_FAILURE_RATE,
                    RF
                ),
                Operation::Set(dec!(0.8)),
            )
        } else {
            let rewards_reduction = (*failure_rate - MIN_FAILURE_RATE)
                / (MAX_FAILURE_RATE - MIN_FAILURE_RATE)
                * MAX_REWARDS_REDUCTION;
            self.logger.log(LogEntry::RewardsReductionPercent {
                failure_rate: *failure_rate,
                min_fr: MIN_FAILURE_RATE,
                max_fr: MAX_FAILURE_RATE,
                max_rr: MAX_REWARDS_REDUCTION,
                rewards_reduction,
            });

            rewards_reduction
        }
    }


    /// Calculates node multipliers for this provider.
    pub fn run(self) -> (NodesMultiplier, Logger) {

        self.step_1_calculate_relative_failure_rates();

        let extrapolated_failure_rate = self.step_2_extrapolate_failure_rate();

        self.step_3_replace_undefined_failure_rates(extrapolated_failure_rate);

        self.logger.log(
            LogEntry::FinalExtrapolatedFailureRates(
                print_failure_rates(&self.nodes_failure_rates())
            ),
        );

        let multiplier_per_node: HashMap<NodeId, Decimal> = self.step_4_compute_multiplier_per_node();

        drop(self.calculator);
        (multiplier_per_node, Rc::unwrap_or_clone(self.logger))
    }
}

fn print_failure_rates(failure_rates: &HashMap<NodeId, Vec<DailyFailureRate>>) -> Table {
    let mut table = Table::new("{:<} {:<} {:<}");

    table.add_row(Row::new()
        .with_cell("Node ID")
        .with_cell("Timestamp")
        .with_cell("Failure Rate")
    );

    // Data Rows
    for (node_id, rates) in failure_rates {
        for rate in rates {
            table.add_row(Row::new()
                .with_cell(node_id.to_string())
                .with_cell(rate.ts)
                .with_cell(format!("{:?}", rate.value))
            );
        }
    }

    table.to_owned()
}
