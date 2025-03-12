use ic_base_types::NodeId;
use itertools::Itertools;
use num_traits::Zero;
use rust_decimal::Decimal;
use std::fmt;
use tabled::Table;

pub fn round_dp_4(dec: &Decimal) -> Decimal {
    dec.round_dp(4)
}

/// Represents an operation that can be executed on Decimals.
/// This is used to run and log the operations that are executed in the library.
#[derive(Debug)]
pub enum Operation {
    Sum(Vec<Decimal>),
    Avg(Vec<Decimal>),
    Divide(Decimal, Decimal),
    Set(Decimal),
    Multiply(Decimal, Decimal),
    SumOps(Vec<Operation>),
}

impl Operation {
    fn sum(operators: &[Decimal]) -> Decimal {
        operators.iter().fold(Decimal::zero(), |acc, val| acc + val)
    }
    fn format_values<T: fmt::Display>(items: &[T], prefix: &str) -> String {
        if items.is_empty() {
            "0".to_string()
        } else {
            format!("{}({})", prefix, items.iter().map(|item| format!("{}", item)).join(","),)
        }
    }

    pub fn execute(&self) -> Decimal {
        match self {
            Operation::Divide(o1, o2) => o1 / o2,
            Operation::Sum(operators) => Self::sum(operators),
            Operation::Avg(operators) => operators.iter().sum::<Decimal>() / Decimal::from(operators.len().max(1)),
            Operation::Set(o1) => *o1,
            Operation::Multiply(o1, o2) => o1 * o2,
            Operation::SumOps(operations) => Self::sum(&operations.iter().map(|operation| operation.execute()).collect_vec()),
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Operation::Sum(values) => write!(f, "{}", Operation::format_values(values, "sum")),
            Operation::Avg(values) => write!(f, "{}", Operation::format_values(values, "avg")),
            Operation::Divide(o1, o2) => write!(f, "{} {} {}", round_dp_4(o1), "/", round_dp_4(o2)),
            Operation::Set(o1) => write!(f, "set {}", o1),
            Operation::Multiply(o1, o2) => write!(f, "{} {} {}", round_dp_4(o1), "*", round_dp_4(o2)),
            Operation::SumOps(operations) => write!(f, "{}", Operation::format_values(operations, "sum")),
        }
    }
}

pub enum LogEntry {
    /// An executed [Operation] with the reason for the operation and the result.
    Execute {
        reason: String,
        operation: Operation,
        result: Decimal,
    },
    NodesMultiplierStep(&'static str),
    Summary(NodeId, Box<Table>),
    PerformanceMultiplier {
        node_id: NodeId,
        failure_rate_in_period: Decimal,
        rewards_reduction: Decimal,
        multiplier: Decimal,
    },
}

impl fmt::Display for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogEntry::Execute { reason, operation, result } => {
                write!(f, "{}: {} = {}", reason, operation, round_dp_4(result))
            }
            LogEntry::Summary(node_id, table) => write!(f, "Summary for Node {}: \n{}", node_id, table),
            LogEntry::NodesMultiplierStep(function_name) => {
                // Format the function name to be more human-readable
                // e.g. "compute_extrapolated_failure_rate" -> "Compute Extrapolated Failure Rate"
                let formatted_str = function_name
                    .replace('_', " ")
                    .split_whitespace()
                    .map(|word| {
                        // Capitalize the first letter of each word
                        let mut chars = word.chars();
                        chars.next().map(|c| c.to_uppercase().collect::<String>()).unwrap_or_default() + chars.as_str()
                    })
                    .collect::<Vec<_>>()
                    .join(" ");

                write!(f, "Compute Rewards Multiplier - Step: {}", formatted_str)
            }
            LogEntry::PerformanceMultiplier {
                node_id,
                failure_rate_in_period: failure_rate,
                rewards_reduction,
                multiplier: rewards_multiplier,
            } => {
                write!(
                    f,
                    "\t{}: failure rate in period: {}, rewards reduction: {} -> Rewards Multiplier: [{}]",
                    node_id,
                    round_dp_4(failure_rate),
                    round_dp_4(rewards_reduction),
                    round_dp_4(rewards_multiplier)
                )
            }
        }
    }
}

#[derive(Default)]
pub struct Logger {
    pub entries: Vec<LogEntry>,
}
impl Logger {
    pub fn log(&mut self, entry: LogEntry) {
        self.entries.push(entry);
    }
    pub fn run_and_log(&mut self, description: &str, operation: Operation) -> Decimal {
        let result = operation.execute();
        self.log(LogEntry::Execute {
            reason: format!("\t{}", description),
            operation,
            result,
        });
        result
    }
}
