use crate::execution_context::{NodeResult, ResultKey, SingleResult};
use crate::metrics::{NodeDailyFailureRate, NodeFailureRate};
use crate::reward_period::TimestampNanos;
use crate::types::RewardableNode;
use chrono::DateTime;
use ic_base_types::NodeId;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, VecDeque};
use tabled::builder::Builder;
use tabled::settings::object::Rows;
use tabled::settings::style::{HorizontalLine, LineText};
use tabled::settings::{Alignment, Border, Span, Style, Theme};
use tabled::Table;
use tabled::Tabled;

#[derive(Clone, Debug, Tabled)]
pub struct DailyNodeFailureRateTabled {
    #[tabled(rename = "Day (UTC)")]
    pub utc_day: String,
    #[tabled(rename = "Original Failure Rate [OFR]")]
    pub original_fr: String,
    #[tabled(rename = "Subnet Assigned")]
    pub subnet_assigned: String,
    #[tabled(rename = "Subnet Failure Rate [SFR]")]
    pub subnet_fr: String,
    #[tabled(rename = "Relative Failure Rate [RFR]")]
    pub relative_fr: String,
    #[tabled(rename = "Extrapolated Failure Rate [EFR]")]
    pub extrapolated_fr: String,
}

impl From<NodeDailyFailureRate> for DailyNodeFailureRateTabled {
    fn from(node_daily_fr: NodeDailyFailureRate) -> Self {
        fn timestamp_to_utc_date(ts: TimestampNanos) -> String {
            DateTime::from_timestamp(ts as i64 / 1_000_000_000, 0)
                .unwrap()
                .naive_utc()
                .format("%d-%m-%Y")
                .to_string()
        }

        let utc_day = timestamp_to_utc_date(node_daily_fr.ts);
        match &node_daily_fr.value {
            NodeFailureRate::DefinedRelative {
                subnet_assigned,
                original_failure_rate,
                subnet_failure_rate,
                value,
            } => Self {
                utc_day,
                original_fr: original_failure_rate.round_dp(4).to_string(),
                subnet_assigned: subnet_assigned.to_string(),
                subnet_fr: subnet_failure_rate.round_dp(4).to_string(),
                relative_fr: value.round_dp(4).to_string(),
                extrapolated_fr: "-".to_string(),
            },
            NodeFailureRate::Extrapolated(value) => Self {
                utc_day,
                original_fr: "N/A".to_string(),
                subnet_assigned: "N/A".to_string(),
                subnet_fr: "N/A".to_string(),
                relative_fr: "N/A".to_string(),
                extrapolated_fr: value.round_dp(4).to_string(),
            },
            _ => panic!("Unexpected NodeFailureRate variant"),
        }
    }
}

pub fn failure_rates_tabled(failure_rates: &BTreeMap<NodeId, Vec<NodeDailyFailureRate>>) -> Vec<Table> {
    fn condense_entries(entries: Vec<DailyNodeFailureRateTabled>) -> Vec<DailyNodeFailureRateTabled> {
        let mut condensed: Vec<DailyNodeFailureRateTabled> = Vec::new();
        let mut queue: VecDeque<DailyNodeFailureRateTabled> = VecDeque::from(entries);

        while let Some(mut start) = queue.pop_front() {
            let mut end_date = start.utc_day.clone();

            while let Some(next) = queue.front() {
                if start.original_fr == next.original_fr
                    && start.subnet_assigned == next.subnet_assigned
                    && start.subnet_fr == next.subnet_fr
                    && start.extrapolated_fr == next.extrapolated_fr
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

    failure_rates
        .iter()
        .map(|(node_id, failure_rates)| {
            let border_text = format!("Node: {} ", node_id);
            let data_tabled: Vec<DailyNodeFailureRateTabled> = failure_rates.iter().map(|fr| fr.clone().into()).collect::<Vec<_>>();

            Table::new(condense_entries(data_tabled))
                .with(Style::modern())
                .with(Alignment::center())
                .with(LineText::new(border_text, Rows::first()).offset(2))
                .to_owned()
        })
        .collect()
}

pub struct NodesComputationTabledResult {
    pub legend: Table,
    pub computation: Table,
}

#[derive(Clone, Debug, Tabled)]
struct LegendTabled {
    #[tabled(rename = "Steps")]
    pub step: String,
    #[tabled(rename = "Description")]
    pub description: String,
}

#[derive(Default)]
pub struct NodesComputationTableBuilder {
    nodes: Vec<NodeId>,
    table_builder: Builder,
    step_counter: usize,
    header_counter: usize,
    col_to_expand: Vec<usize>,
    legend: Vec<LegendTabled>,
}

impl NodesComputationTableBuilder {
    pub fn new(nodes: Vec<RewardableNode>) -> Self {
        let mut builder = Self::default();

        let mut nodes_id_col = vec!["Node ID".to_string()];
        let mut nodes_type_col = vec!["Node Type".to_string()];
        let mut nodes_region_col = vec!["Node Region".to_string()];

        for node in nodes {
            builder.nodes.push(node.node_id);

            nodes_id_col.push(node.node_id.to_string());
            nodes_type_col.push(node.node_type.to_string());
            nodes_region_col.push(node.region.to_string());
        }

        builder.add_column(nodes_id_col);
        builder.add_column(nodes_type_col);
        builder.add_column(nodes_region_col);
        builder.header_counter += 3;

        builder
    }

    fn add_column(&mut self, col: Vec<String>) {
        self.table_builder.push_column(col);
    }

    fn new_column(&mut self, key: ResultKey) -> Vec<String> {
        self.step_counter += 1;
        let step = format!("Step {}", self.step_counter);
        self.legend.push(LegendTabled {
            step: step.clone(),
            description: key.description().to_string(),
        });

        vec![step]
    }

    pub fn with_node_result_column(&mut self, key: NodeResult, values: BTreeMap<NodeId, Decimal>) {
        let mut column = self.new_column(key.into());
        for node_id in &self.nodes {
            if let Some(value) = values.get(node_id) {
                if matches!(key, NodeResult::BaseRewards | NodeResult::AdjustedRewards) {
                    column.push(myr_xdr(value));
                } else {
                    column.push(round(value));
                }
            } else {
                column.push("-".to_string());
            }
        }
        if matches!(key, NodeResult::BaseRewards) {
            self.table_builder.insert_column(self.header_counter, column);
            self.header_counter += 1;
        } else {
            self.add_column(column);
        }
    }

    pub fn with_single_result_column(&mut self, key: SingleResult, value: Decimal) {
        let mut column = self.new_column(key.into());
        self.col_to_expand.push(self.step_counter);

        if matches!(key, SingleResult::RewardsTotal) {
            column.push(myr_xdr(&value));
        } else {
            column.push(round(&value));
        }

        self.add_column(column);
    }

    fn build_legend(&self) -> Table {
        let mut legend_theme = Theme::from_style(Style::modern());
        let hline = HorizontalLine::inherit(Style::modern()).left(Border::inherit(Style::modern()).get_left());
        legend_theme.remove_horizontal_lines();
        legend_theme.insert_horizontal_line(1, hline);

        Table::new(&self.legend)
            .with(legend_theme)
            .with(LineText::new("Legend", Rows::first()).offset(2))
            .to_owned()
    }

    pub fn build(self) -> NodesComputationTabledResult {
        let legend_table = &self.build_legend();

        let mut table = self
            .table_builder
            .build()
            .with(Style::modern())
            .with(Alignment::center())
            .with(LineText::new("Nodes Computation", Rows::first()).offset(2))
            .to_owned();

        for idx in self.col_to_expand {
            let col_idx = idx + self.header_counter - 1;
            table.modify((1, col_idx), Span::row(isize::MAX));
        }

        NodesComputationTabledResult {
            legend: legend_table.clone(),
            computation: table,
        }
    }
}

pub fn myr_xdr(value: &Decimal) -> String {
    format!("{} myrXDR", value.round())
}

pub fn round(value: &Decimal) -> String {
    value.round_dp(4).to_string()
}
