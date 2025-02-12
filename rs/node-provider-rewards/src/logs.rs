use std::collections::HashMap;
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use itertools::Itertools;
use rust_decimal::{prelude::Zero, Decimal};
use std::fmt;
use chrono::NaiveDate;
use crate::types::SystematicFailureRate;

fn round_dp_4(dec: &Decimal) -> Decimal {
    dec.round_dp(4)
}

#[derive(Clone, PartialEq, Eq, Debug)]
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
            format!(
                "{}({})",
                prefix,
                items.iter().map(|item| format!("{}", item)).join(","),
            )
        }
    }

    fn execute(&self) -> Decimal {
        match self {
            Operation::Sum(operators) => Self::sum(operators),
            Operation::Avg(operators) => {
                Self::sum(operators) / Decimal::from(operators.len().max(1))
            }
            Operation::Subtract(o1, o2) => o1 - o2,
            Operation::Divide(o1, o2) => o1 / o2,
            Operation::Multiply(o1, o2) => o1 * o2,
            Operation::Set(o1) => *o1,
            Operation::SumOps(operations) => Self::sum(
                &operations
                    .iter()
                    .map(|operation| operation.execute())
                    .collect_vec(),
            ),
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
            Operation::SumOps(operations) => {
                return write!(f, "{}", Operation::format_values(operations, "sum"))
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

#[derive(PartialEq, Eq, Debug)]
pub enum FailureRateStatus {
    Undefined,
    Extrapolated {
        failure_rate: Decimal,
    },
    Defined{
        subnet_assigned: SubnetId,
        failure_rate: Decimal,
    },
    DefinedRelative {
        subnet_assigned: SubnetId,
        original_failure_rate: Decimal,
        systematic_failure_rate: Decimal,
        relative_failure_rate: Decimal,
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct DailyFailureRate {
    pub ts: u64,
    pub status: FailureRateStatus
}


#[derive(PartialEq, Eq, Debug)]
pub enum LogEntry {
    Execute {
        reason: String,
        operation: Operation,
        result: Decimal,
    },
    ComputeRelativeFailureRates(HashMap<NodeId, Vec<DailyFailureRate>>),
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
            }
            LogEntry::RewardsXDRTotal(rewards_xdr_total, rewards_xdr_total_adjusted) => {
                write!(
                    f,
                    "Total rewards XDR permyriad: {}\nTotal rewards XDR permyriad not adjusted: {}",
                    round_dp_4(rewards_xdr_total),
                    round_dp_4(rewards_xdr_total_adjusted)
                )
            }
            LogEntry::RateNotFoundInRewardTable { node_type, region } => {
                write!(
                    f,
                    "RateNotFoundInRewardTable | node_type: {}, region: {}",
                    node_type, region
                )
            }
            LogEntry::RewardTableEntry {
                node_type,
                region,
                coeff,
                base_rewards,
                node_count,
            } => {
                write!(
                    f,
                    "node_type: {}, region: {}, coeff: {}, base_rewards: {}, node_count: {}",
                    node_type, region, coeff, base_rewards, node_count
                )
            }
            LogEntry::ActiveIdiosyncraticFailureRates {
                node_id,
                failure_rates,
            } => {
                write!(
                    f,
                    "ActiveIdiosyncraticFailureRates | node_id={}, failure_rates_discounted={:?}",
                    node_id, failure_rates
                )
            }
            LogEntry::ComputeRewardsForNode {
                node_id,
                node_type,
                region,
            } => {
                write!(
                    f,
                    "Compute Rewards For Node | node_id={}, node_type={}, region={}",
                    node_id, node_type, region
                )
            }
            LogEntry::CalculateRewardsForNodeProvider(node_provider_id) => {
                write!(
                    f,
                    "CalculateRewardsForNodeProvider | node_provider_id={}",
                    node_provider_id
                )
            }
            LogEntry::BaseRewards(rewards_xdr) => {
                write!(f, "Base rewards XDRs: {}", round_dp_4(rewards_xdr))
            }
            LogEntry::IdiosyncraticFailureRates(failure_rates) => {
                write!(
                    f,
                    "Idiosyncratic daily failure rates : {}",
                    failure_rates.iter().join(",")
                )
            }
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
            }
            LogEntry::ComputeBaseRewardsForRegionNodeType => {
                write!(f, "Compute Base Rewards For RegionNodeType")
            }
            LogEntry::ComputeUnassignedFailureRate => {
                write!(f, "Compute Unassigned Days Failure Rate")
            }
            LogEntry::NodeStatusAssigned => {
                write!(f, "Node status: Assigned")
            }
            LogEntry::NodeStatusUnassigned => {
                write!(f, "Node status: Unassigned")
            }
        }
    }
}


#[derive(Default, PartialEq, Eq, Debug)]
pub struct NodeProviderRewardsLog {
    entries: Vec<(LogLevel, LogEntry)>,
}

impl NodeProviderRewardsLog {
    pub fn add_entry(&mut self, entry: LogEntry) {
        self.entries.push((log_level, entry));
    }

    pub fn execute(&mut self, reason: &str, operation: Operation) -> Decimal {
        let result = operation.execute();
        let entry = LogEntry::Execute {
            reason: reason.to_string(),
            operation,
            result,
        };
        self.add_entry(LogLevel::Mid, entry);
        result
    }

    pub fn get_log(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|(log_level, entry)| match log_level {
                LogLevel::High => format!("{}", entry),
                LogLevel::Mid => format!("    - {}", entry),
                LogLevel::Low => format!("        - {}", entry),
            })
            .collect_vec()
    }
}
