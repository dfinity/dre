use ic_base_types::PrincipalId;
use itertools::Itertools;
use rust_decimal::{prelude::Zero, Decimal};
use std::fmt;

pub enum Operation {
    Sum(Vec<Decimal>),
    Avg(Vec<Decimal>),
    Subtract(Decimal, Decimal),
    Multiply(Decimal, Decimal),
    Divide(Decimal, Decimal),
    Set(Decimal),
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

    fn execute(&self) -> Decimal {
        match self {
            Operation::Sum(operators) => Self::sum(operators),
            Operation::Avg(operators) => Self::sum(operators) / Decimal::from(operators.len().max(1)),
            Operation::Subtract(o1, o2) => o1 - o2,
            Operation::Divide(o1, o2) => o1 / o2,
            Operation::Multiply(o1, o2) => o1 * o2,
            Operation::Set(o1) => *o1,
            Operation::SumOps(operations) => Self::sum(&operations.iter().map(|operation| operation.execute()).collect_vec()),
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (symbol, o1, o2) = match self {
            Operation::Sum(values) => return write!(f, "{}", Operation::format_values(values, "sum")),
            Operation::SumOps(operations) => return write!(f, "{}", Operation::format_values(operations, "sum")),
            Operation::Avg(values) => return write!(f, "{}", Operation::format_values(values, "avg")),
            Operation::Subtract(o1, o2) => ("-", o1, o2),
            Operation::Divide(o1, o2) => ("/", o1, o2),
            Operation::Multiply(o1, o2) => ("*", o1, o2),
            Operation::Set(o1) => return write!(f, "set {}", o1),
        };
        write!(f, "{} {} {}", o1.round_dp(4), symbol, o2.round_dp(4))
    }
}

pub enum LogEntry {
    RewardsForNodeProvider(PrincipalId),
    ComputeRewardMultiplierForNode(PrincipalId),
    RewardsXDRTotal(Decimal),
    ExecuteOperation {
        reason: String,
        operation: Operation,
        result: Decimal,
    },
    RewardablesInRegionNodeType {
        node_type: String,
        region: String,
        count: usize,
        assigned_multipliers: Vec<Decimal>,
        unassigned_multipliers: Vec<Decimal>,
    },
    RateNotFoundInRewardTable {
        node_type: String,
        region: String,
    },
    RewardTableEntry {
        node_type: String,
        region: String,
        coeff: Decimal,
        base_rewards: Decimal,
    },
    AvgType3Rewards(String, Decimal),
}

impl fmt::Display for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogEntry::ExecuteOperation { reason, operation, result } => {
                write!(f, "ExecuteOperation: reason={}, operation={}, result={}", reason, operation, result)
            }
            LogEntry::RewardsForNodeProvider(principal) => {
                write!(f, "RewardsForNodeProvider: principal={}", principal)
            }
            LogEntry::ComputeRewardMultiplierForNode(principal) => {
                write!(f, "RewardsMultiplier: principal={}", principal)
            }
            LogEntry::RewardsXDRTotal(rewards_xdr_total) => {
                write!(f, "Total rewards XDR permyriad: {}", rewards_xdr_total)
            }
            LogEntry::RateNotFoundInRewardTable { node_type, region } => {
                write!(f, "RateNotFoundInRewardTable: node_type={}, region={}", node_type, region)
            }
            LogEntry::RewardTableEntry {
                node_type,
                region,
                coeff,
                base_rewards,
            } => {
                write!(
                    f,
                    "RewardTableEntry: node_type={}, region={}, coeff={}, base_rewards={}",
                    node_type, region, coeff, base_rewards
                )
            }
            LogEntry::RewardablesInRegionNodeType {
                node_type,
                region,
                count,
                assigned_multipliers: assigned_multiplier,
                unassigned_multipliers: unassigned_multiplier,
            } => {
                write!(
                    f,
                    "Region {} with type: {} | Rewardable Nodes: {} Assigned Multipliers: {:?} Unassigned Multipliers: {:?}",
                    node_type, region, count, assigned_multiplier, unassigned_multiplier
                )
            }
            LogEntry::AvgType3Rewards(region, avg_rewards) => write!(f, "Avg. rewards for nodes with type: type3* in region: {} is {}", region, avg_rewards),
        }
    }
}

#[derive(Default)]
pub struct RewardsPerNodeProviderLog {
    entries: Vec<LogEntry>,
}

impl RewardsPerNodeProviderLog {
    pub fn add_entry(&mut self, entry: LogEntry) {
        self.entries.push(entry);
    }

    pub fn execute(&mut self, reason: &str, operation: Operation) -> Decimal {
        let result = operation.execute();
        let entry = LogEntry::ExecuteOperation {
            reason: reason.to_string(),
            operation,
            result,
        };
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
