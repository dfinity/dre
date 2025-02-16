use std::cell::RefCell;
use ic_base_types::NodeId;
use itertools::Itertools;
use rust_decimal::{prelude::Zero, Decimal};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;
use tabular::Table;

fn round_dp_4(dec: &Decimal) -> Decimal {
    dec.round_dp(4)
}

#[derive(Debug)]
pub enum Operation {
    Sum(Vec<Decimal>),
    Avg(Vec<Decimal>),
    Subtract(Decimal, Decimal),
    Multiply(Decimal, Decimal),
    Divide(Decimal, Decimal),
    Set(Decimal),
}

impl Operation {
    fn format_values<T: fmt::Display>(items: &[T], prefix: &str) -> String {
        if items.is_empty() {
            "0".to_string()
        } else {
            format!(
                "{}({})",
                prefix,
                items.iter().map(|item| format!("{}", item)).join(","),
            )
        }
    }

    fn execute(&self) -> Decimal {
        match self {
            Operation::Sum(operators) => operators.iter().sum(),
            Operation::Avg(operators) => {
                operators.iter().sum::<Decimal>() / Decimal::from(operators.len().max(1))
            }
            Operation::Subtract(o1, o2) => o1 - o2,
            Operation::Divide(o1, o2) => o1 / o2,
            Operation::Multiply(o1, o2) => o1 * o2,
            Operation::Set(o1) => *o1
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (symbol, o1, o2) = match self {
            Operation::Sum(values) => {
                return write!(
                    f,
                    "{}",
                    Operation::format_values(&values.iter().map(round_dp_4).collect_vec(), "sum")
                )
            }
            Operation::Avg(values) => {
                return write!(
                    f,
                    "{}",
                    Operation::format_values(&values.iter().map(round_dp_4).collect_vec(), "avg")
                )
            }
            Operation::Subtract(o1, o2) => ("-", o1, o2),
            Operation::Divide(o1, o2) => ("/", o1, o2),
            Operation::Multiply(o1, o2) => ("*", o1, o2),
            Operation::Set(o1) => return write!(f, "set {}", o1),
        };

        write!(f, "{} {} {}", round_dp_4(o1), symbol, round_dp_4(o2))
    }
}


pub enum LogEntry {
    Execute {
        reason: String,
        operation: Operation,
        result: Decimal,
    },
    RewardsReductionPercent {
        failure_rate: Decimal,
        min_fr: Decimal,
        max_fr: Decimal,
        max_rr: Decimal,
        rewards_reduction: Decimal,
    },
    FinalExtrapolatedFailureRates(Table),
    ComputeNodeMultiplier(NodeId),
    FinalNodesMultiplier(HashMap<NodeId, Decimal>),
    ComputeFailureRateExtrapolation,
}

impl fmt::Display for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogEntry::Execute {
                reason,
                operation,
                result,
            } => {
                write!(f, "{}: {} = {}", reason, operation, round_dp_4(result))
            },
            LogEntry::RewardsReductionPercent {
                failure_rate,
                min_fr,
                max_fr,
                max_rr,
                rewards_reduction,
            } => {
                write!(
                    f,
                    "Rewards reduction percent: ({} - {}) / ({} - {}) * {} = {}",
                    round_dp_4(failure_rate),
                    min_fr,
                    max_fr,
                    min_fr,
                    max_rr,
                    round_dp_4(rewards_reduction)
                )
            },
            LogEntry::FinalExtrapolatedFailureRates(_) => write!(f, "FinalExtrapolatedFailureRates"),
            LogEntry::ComputeNodeMultiplier(_) => write!(f, "ComputeNodeMultiplier"),
            LogEntry::FinalNodesMultiplier(_) => write!(f, "FinalNodesMultiplier"),
            LogEntry::ComputeFailureRateExtrapolation => write!(f, "ComputeFailureRateExtrapolation"),
        }
    }
}

#[derive(Default)]
pub struct Logger {
    entries: Rc<RefCell<Vec<LogEntry>>>,
}

impl Logger {
    pub fn log(&self, entry: LogEntry) {
        self.entries.borrow_mut().push(entry);
    }

    pub fn get_logs(&self) -> Vec<String> {
        self.entries.borrow().iter().map(|entry| format!("{}", entry)).collect()
    }
}

pub struct OperationCalculator {
    logger: Option<Rc<Logger>>,
}


impl OperationCalculator {
    pub fn new() -> Self {
        Self {
            logger: None,
        }
    }
    pub fn with_logger(&self, logger: Rc<Logger>) -> Self {
        Self {
            logger: Some(logger),
        }
    }

    pub fn run(&self, reason: &str, operation: Operation) -> Decimal {
        let result = operation.execute();
        if let Some(logger) = self.logger.as_ref() {
            logger.log(LogEntry::Execute {
                reason: reason.to_string(),
                operation,
                result,
            });
        }

        result
    }
}
