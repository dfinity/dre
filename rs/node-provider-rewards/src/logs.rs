use ic_base_types::NodeId;
use itertools::Itertools;
use rust_decimal::Decimal;
use std::fmt;
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
            format!("{}({})", prefix, items.iter().map(|item| format!("{}", item)).join(","),)
        }
    }

    pub fn execute(&self) -> Decimal {
        match self {
            Operation::Sum(operators) => operators.iter().sum(),
            Operation::Avg(operators) => operators.iter().sum::<Decimal>() / Decimal::from(operators.len().max(1)),
            Operation::Subtract(o1, o2) => o1 - o2,
            Operation::Divide(o1, o2) => o1 / o2,
            Operation::Multiply(o1, o2) => o1 * o2,
            Operation::Set(o1) => *o1,
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (symbol, o1, o2) = match self {
            Operation::Sum(values) => return write!(f, "{}", Operation::format_values(&values.iter().map(round_dp_4).collect_vec(), "sum")),
            Operation::Avg(values) => return write!(f, "{}", Operation::format_values(&values.iter().map(round_dp_4).collect_vec(), "avg")),
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
    NodesMultiplierStep(&'static str),
    Summary(Table),
    RewardsMultiplier {
        node_id: NodeId,
        failure_rate: Decimal,
        rewards_reduction: Decimal,
        rewards_multiplier: Decimal,
    },
}

impl fmt::Display for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogEntry::Execute { reason, operation, result } => {
                write!(f, "{}: {} = {}", reason, operation, round_dp_4(result))
            }
            LogEntry::Summary(table) => write!(f, "Summary:\n{}", table),
            LogEntry::NodesMultiplierStep(step) => {
                let formatted_str = step
                    .replace('_', " ")
                    .split_whitespace()
                    .map(|word| {
                        let mut chars = word.chars();
                        chars.next().map(|c| c.to_uppercase().collect::<String>()).unwrap_or_default() + chars.as_str()
                    })
                    .collect::<Vec<_>>()
                    .join(" ");

                write!(f, "Compute Rewards Multiplier - Step: {}", formatted_str)
            }
            LogEntry::RewardsMultiplier {
                node_id,
                failure_rate,
                rewards_reduction,
                rewards_multiplier,
            } => {
                write!(
                    f,
                    "Node {}: failure rate {} rewards reduction: {} REWARDS MULTIPLIER {}",
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

    pub fn get_logs(&self) -> Vec<String> {
        self.entries.iter().map(|entry| format!("{}", entry)).collect()
    }
}
