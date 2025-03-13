use crate::logs::round_dp_4;
use crate::metrics::{NodeDailyFailureRate, NodeFailureRate};
use crate::reward_period::TimestampNanos;
use chrono::DateTime;
use std::collections::VecDeque;
use tabled::settings::Style;
use tabled::Table;
use tabled::Tabled;

#[derive(Clone, Debug, Tabled)]
pub struct DailyNodeFailureRateTabled {
    #[tabled(rename = "Day (UTC)")]
    pub utc_day: String,
    #[tabled(rename = "Original Failure Rate")]
    pub original_failure_rate: String,
    #[tabled(rename = "Subnet Assigned")]
    pub subnet_assigned: String,
    #[tabled(rename = "Subnet Failure Rate")]
    pub subnet_failure_rate: String,
    #[tabled(rename = "Relative/Extrapolated Failure Rate")]
    pub final_failure_rate: String,
}

fn timestamp_to_utc_date(ts: TimestampNanos) -> String {
    DateTime::from_timestamp(ts as i64 / 1_000_000_000, 0)
        .unwrap()
        .naive_utc()
        .format("%d-%m-%Y")
        .to_string()
}

impl From<NodeDailyFailureRate> for DailyNodeFailureRateTabled {
    fn from(value: NodeDailyFailureRate) -> Self {
        let utc_day = timestamp_to_utc_date(value.ts);
        let original_failure_rate = match &value.value {
            NodeFailureRate::DefinedRelative { original_failure_rate, .. } => round_dp_4(original_failure_rate).to_string(),
            _ => "N/A".to_string(),
        };
        let subnet_assigned = match &value.value {
            NodeFailureRate::DefinedRelative { subnet_assigned, .. } => subnet_assigned.to_string(),
            _ => "N/A".to_string(),
        };
        let subnet_failure_rate = match &value.value {
            NodeFailureRate::DefinedRelative { subnet_failure_rate, .. } => round_dp_4(subnet_failure_rate).to_string(),
            _ => "N/A".to_string(),
        };
        let final_failure_rate = match value.value {
            NodeFailureRate::DefinedRelative { value, .. } | NodeFailureRate::Extrapolated(value) => round_dp_4(&value).to_string(),
            _ => "N/A".to_string(),
        };

        Self {
            utc_day,
            original_failure_rate,
            subnet_assigned,
            subnet_failure_rate,
            final_failure_rate,
        }
    }
}

fn condense_table(entries: Vec<DailyNodeFailureRateTabled>) -> Vec<DailyNodeFailureRateTabled> {
    let mut condensed: Vec<DailyNodeFailureRateTabled> = Vec::new();
    let mut queue: VecDeque<DailyNodeFailureRateTabled> = VecDeque::from(entries);

    while let Some(mut start) = queue.pop_front() {
        let mut end_date = start.utc_day.clone();

        while let Some(next) = queue.front() {
            if start.original_failure_rate == next.original_failure_rate
                && start.subnet_assigned == next.subnet_assigned
                && start.subnet_failure_rate == next.subnet_failure_rate
                && start.final_failure_rate == next.final_failure_rate
            {
                end_date = next.utc_day.clone();
                queue.pop_front();
            } else {
                break;
            }
        }

        if start.utc_day != end_date {
            start.utc_day = format!("{} to {}", start.utc_day, end_date);
        }

        condensed.push(start);
    }

    condensed
}

pub fn generate_table_summary(daily_fr: Vec<NodeDailyFailureRate>) -> Table {
    let data_tabled: Vec<DailyNodeFailureRateTabled> = daily_fr.into_iter().map(|fr| fr.into()).collect::<Vec<_>>();

    Table::new(condense_table(data_tabled)).with(Style::sharp()).to_owned()
}
