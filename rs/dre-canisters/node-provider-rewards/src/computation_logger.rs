use std::fmt;

use ic_base_types::PrincipalId;
use num_traits::Zero;
use rust_decimal::Decimal;

pub enum Operation {
    Sum(Vec<Decimal>),
    Subtract(Decimal, Decimal),
    Multiply(Decimal, Decimal),
    Divide(Decimal, Decimal),
    Set(Decimal),
}

impl Operation {
    fn execute(&self) -> Decimal {
        match self {
            Operation::Sum(operators) => operators.iter().cloned().fold(Decimal::zero(), |acc, val| acc + val),
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
            Operation::Sum(values) => {
                return if values.is_empty() {
                    write!(f, "0")
                } else {
                    write!(
                        f,
                        "{} + {}",
                        values[0],
                        values[1..].iter().map(|o| format!("{}", o.round_dp(4))).collect::<Vec<_>>().join(" + ")
                    )
                }
            }
            Operation::Subtract(o1, o2) => ("-", o1, o2),
            Operation::Divide(o1, o2) => ("/", o1, o2),
            Operation::Multiply(o1, o2) => ("*", o1, o2),
            Operation::Set(o1) => return write!(f, "set {}", o1),
        };
        write!(f, "{} {} {}", o1.round_dp(4), symbol, o2.round_dp(4))
    }
}

pub enum Entry {
    ExecuteOperation { reason: String, operation: Operation, result: Decimal },
    RewardsForNodeProvider(PrincipalId),
    RewardsMultiplier(PrincipalId),
    RateNotFoundInRewardTable{ node_type: String, region: String},
    Type3NodesCoefficientsRewards { node_type: String, region: String, coeff: Decimal, base_rewards: Decimal },
    OtherNodesRewards { node_type: String, region: String, base_rewards: Decimal },
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Entry::ExecuteOperation { reason, operation, result } => {
                write!(f, "ExecuteOperation: reason={}, operation={}, result={}", reason, operation, result)
            }
            Entry::RewardsForNodeProvider(principal) => {
                write!(f, "RewardsForNodeProvider: principal={}", principal)
            }
            Entry::RewardsMultiplier(principal) => {
                write!(f, "RewardsMultiplier: principal={}", principal)
            }
            Entry::RateNotFoundInRewardTable { node_type, region } => {
                write!(f, "RateNotFoundInRewardTable: node_type={}, region={}", node_type, region)
            }
            Entry::Type3NodesCoefficientsRewards { node_type, region, coeff, base_rewards } => {
                write!(f, "Type3NodesCoefficientsRewards: node_type={}, region={}, coeff={}, base_rewards={}",
                    node_type, region, coeff, base_rewards)
            }
            Entry::OtherNodesRewards { node_type, region, base_rewards } => {
                write!(f, "OtherNodesRewards: node_type={}, region={}, base_rewards={}", node_type, region, base_rewards)
            }
        }
    }
}

pub struct RewardsComputationLogger {
    pub entries: Vec<Entry>,
}

impl RewardsComputationLogger {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn add_entry(&mut self, entry: Entry) {
        ic_cdk::println!("{}", entry);

        self.entries.push(entry);
    }

    pub fn flush_log_entries(&mut self) {
        self.entries.clear();
    }

    pub fn execute(&mut self, reason: &str, operation: Operation) -> Decimal {
        let result = operation.execute();
        let entry = Entry::ExecuteOperation {
            reason: reason.to_string(),
            operation,
            result,
        };

        ic_cdk::println!("{}", entry);

        self.entries.push(entry);
        result
    }

    pub fn get_log(&self) -> String {
        let mut log = Vec::new();

        for (index, entry) in self.entries.iter().enumerate() {
            log.push(format!("Entry {}: {} ", index, entry));
        }
        log.join("\n")
    }
}
