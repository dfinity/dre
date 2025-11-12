use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};
use anyhow::anyhow;
use chrono::Datelike;
use chrono::{DateTime, NaiveDate};
use clap::{Args, Subcommand};
use csv::Writer;
use futures_util::future::join_all;
use ic_base_types::{PrincipalId, SubnetId};
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_canisters::node_rewards::NodeRewardsCanisterWrapper;
use ic_node_rewards_canister_api::DateUtc;
use ic_node_rewards_canister_api::provider_rewards_calculation::{DailyNodeFailureRate, DailyNodeProviderRewards, DailyResults};
use itertools::Itertools;
use log::info;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use tabled::Tabled;
use url::Url;

#[derive(Subcommand, Debug, Clone)]
pub enum NodeRewardsMode {
    /// Show ongoing rewards from the latest governance snapshot timestamp to yesterday
    Ongoing {
        /// If set, write detailed CSVs to this directory
        #[arg(long)]
        csv_detailed_output_path: Option<String>,

        /// Filter to a single provider (full principal or provider prefix)
        #[arg(long)]
        provider_id: Option<String>,
    },
    /// Show past rewards for a given month (YYYY-MM) and compare with governance
    PastRewards {
        /// Month in format YYYY-MM
        month: String,
        /// If set, write detailed CSVs to this directory
        #[arg(long)]
        csv_detailed_output_path: Option<String>,
        /// Filter to a single provider (full principal or provider prefix)
        #[arg(long)]
        provider_id: Option<String>,
        /// If set, display comparison table with governance rewards
        #[arg(long)]
        compare_with_governance: bool,
    },
    /// Push node rewards metrics to VictoriaMetrics for a specific date
    PushToVictoria {
        /// Date in format YYYY-MM-DD
        #[arg(long)]
        date: Option<String>,
        /// VictoriaMetrics URL (defaults to http://localhost:9090)
        #[arg(long)]
        victoria_url: Option<String>,
    },
}

#[derive(Args, Debug)]
pub struct NodeRewards {
    /// Subcommand mode: ongoing or past-rewards <month>
    #[command(subcommand)]
    pub mode: NodeRewardsMode,

    #[arg(skip)]
    pub node_rewards_ctx: RefCell<Option<NodeRewardsCtx>>,
}

#[derive(Debug, Clone)]
pub struct NodeRewardsCtx {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub csv_detailed_output_path: Option<String>,
    pub provider_id: Option<String>,
    pub compare_with_governance: bool,
    pub governance_providers_rewards: BTreeMap<PrincipalId, u64>,
}

impl Default for NodeRewards {
    fn default() -> Self {
        Self {
            mode: NodeRewardsMode::Ongoing {
                csv_detailed_output_path: None,
                provider_id: None,
            },
            node_rewards_ctx: RefCell::new(None),
        }
    }
}

impl ExecutableCommand for NodeRewards {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Signer
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let (_, canister_agent) = ctx.create_ic_agent_canister_client().await?;
        let node_rewards_client: NodeRewardsCanisterWrapper = canister_agent.clone().into();
        let governance_client: GovernanceCanisterWrapper = canister_agent.into();

        info!("Started action...");

        let gov_rewards_list = governance_client.list_node_provider_rewards(None).await?;
        // Run the selected subcommand and populate self attributes
        match &self.mode {
            NodeRewardsMode::PushToVictoria { date, victoria_url } => {
                let url_string = victoria_url
                    .clone()
                    .or_else(|| std::env::var("VICTORIA_METRICS_URL").ok())
                    .unwrap_or_else(|| "http://localhost:9090".to_string());

                let victoria_url = Url::parse(&url_string).map_err(|e| anyhow!("Invalid VictoriaMetrics URL '{}': {}", url_string, e))?;

                return self.push_rewards_metrics_to_victoria(ctx, date.clone(), victoria_url).await;
            }
            NodeRewardsMode::Ongoing {
                csv_detailed_output_path,
                provider_id,
            } => {
                let last_rewards = gov_rewards_list.into_iter().next().unwrap();
                // Range: from latest governance ts to yesterday
                let today = chrono::Utc::now().date_naive();
                let yesterday = today.pred_opt().unwrap();
                let end_ts = yesterday
                    .and_hms_opt(0, 0, 0)
                    .ok_or_else(|| anyhow::anyhow!("Invalid yesterday midnight"))?
                    .and_utc()
                    .timestamp();
                let start_date = DateTime::from_timestamp(last_rewards.timestamp as i64, 0).unwrap().date_naive();
                let end_date = DateTime::from_timestamp(end_ts, 0).unwrap().date_naive();

                if end_date < start_date {
                    return Err(anyhow!("Rewards have been distributed today, wait until tomorrow and retry"));
                }

                *self.node_rewards_ctx.borrow_mut() = Some(NodeRewardsCtx {
                    start_date,
                    end_date,
                    csv_detailed_output_path: csv_detailed_output_path.clone(),
                    provider_id: provider_id.clone(),
                    governance_providers_rewards: BTreeMap::new(),
                    compare_with_governance: false,
                });
            }
            NodeRewardsMode::PastRewards {
                month,
                csv_detailed_output_path,
                provider_id,
                compare_with_governance,
            } => {
                let target = chrono::NaiveDate::parse_from_str(&(month.to_string() + "-01"), "%Y-%m-%d")?;
                let mut idx_in_month: Option<usize> = None;
                for (i, snap) in gov_rewards_list.iter().enumerate() {
                    let dt = DateTime::from_timestamp(snap.timestamp as i64, 0)
                        .ok_or_else(|| anyhow::anyhow!("Invalid governance timestamp"))?
                        .date_naive();
                    if dt.year() == target.year() && dt.month() == target.month() {
                        idx_in_month = Some(i);
                        break;
                    }
                }
                let i = idx_in_month.ok_or_else(|| anyhow::anyhow!("No governance snapshot found for {}", month))?;
                let last = &gov_rewards_list[i];
                let xdr_permyriad_per_icp = last
                    .xdr_conversion_rate
                    .as_ref()
                    .and_then(|x| x.xdr_permyriad_per_icp)
                    .ok_or_else(|| anyhow::anyhow!("Missing XDR conversion rate"))?;

                let governance_providers_rewards: BTreeMap<PrincipalId, u64> = last
                    .rewards
                    .iter()
                    .map(|r| {
                        let icp_amount = r.amount_e8s as f64 / 100_000_000f64;
                        let xdr_permyriad_amount = icp_amount * xdr_permyriad_per_icp as f64;
                        (r.node_provider.as_ref().and_then(|np| np.id).unwrap(), xdr_permyriad_amount as u64)
                    })
                    .collect();

                let prev = gov_rewards_list
                    .get(i + 1)
                    .ok_or_else(|| anyhow::anyhow!("Previous governance snapshot not found for {}", month))?;

                let start_date = DateTime::from_timestamp(prev.timestamp as i64, 0)
                    .ok_or_else(|| anyhow::anyhow!("Invalid previous timestamp"))?
                    .date_naive();
                let end_date = DateTime::from_timestamp(last.timestamp as i64, 0)
                    .ok_or_else(|| anyhow::anyhow!("Invalid last timestamp"))?
                    .date_naive()
                    .pred_opt()
                    .ok_or_else(|| anyhow::anyhow!("Cannot get previous day"))?;

                *self.node_rewards_ctx.borrow_mut() = Some(NodeRewardsCtx {
                    start_date,
                    end_date,
                    csv_detailed_output_path: csv_detailed_output_path.clone(),
                    provider_id: provider_id.clone(),
                    governance_providers_rewards,
                    compare_with_governance: *compare_with_governance,
                });
            }
        }

        let nrc_data = self.fetch_nrc_data(&node_rewards_client).await?;

        if let Some(output_path) = self.ctx().csv_detailed_output_path {
            self.generate_csv_files_by_provider(&nrc_data, &output_path).await?;
        } else {
            self.print_daily_summary_console(&nrc_data)?;
        }

        self.print_comparison_console(&nrc_data, self.ctx().compare_with_governance, &self.ctx().governance_providers_rewards)?;

        Ok(())
    }
}

pub struct NrcData {
    pub providers_rewards: BTreeMap<PrincipalId, Vec<(DateUtc, DailyNodeProviderRewards)>>,
    pub subnets_failure_rates: BTreeMap<SubnetId, Vec<(DateUtc, f64)>>,
}

/// Get provider prefix from full principal ID string
fn get_provider_prefix(provider_id_str: &str) -> String {
    provider_id_str.split('-').next().unwrap_or(provider_id_str).to_string()
}

impl NodeRewards {
    fn ctx(&self) -> NodeRewardsCtx {
        self.node_rewards_ctx.borrow().clone().unwrap()
    }

    async fn fetch_nrc_data(&self, node_rewards_client: &NodeRewardsCanisterWrapper) -> anyhow::Result<NrcData> {
        let start_date = self.ctx().start_date;
        let end_date = self.ctx().end_date;

        println!("Fetching node rewards for all providers from NRC from {} to {}...", start_date, end_date);

        let days: Vec<DateUtc> = start_date.iter_days().take_while(|day| day <= &end_date).map(DateUtc::from).collect();
        let responses: Vec<anyhow::Result<DailyResults>> =
            join_all(days.iter().map(|day| async move { node_rewards_client.get_rewards_daily(*day).await })).await;

        let mut providers_rewards: BTreeMap<PrincipalId, Vec<(DateUtc, DailyNodeProviderRewards)>> = BTreeMap::new();
        let mut subnets_failure_rates: BTreeMap<SubnetId, Vec<(DateUtc, f64)>> = BTreeMap::new();

        for (day, response) in days.into_iter().zip(responses.into_iter()) {
            match response {
                Ok(daily_results) => {
                    for (provider_id, provider_rewards) in daily_results.provider_results {
                        providers_rewards.entry(provider_id).or_default().push((day, provider_rewards));
                    }

                    for (subnet_id, failure_rate) in daily_results.subnets_failure_rate {
                        subnets_failure_rates.entry(subnet_id).or_default().push((day, failure_rate));
                    }
                }
                Err(e) => {
                    println!("Error fetching node rewards for provider: {}", e);
                }
            }
        }

        if let Some(provider_filter) = self.ctx().provider_id {
            providers_rewards.retain(|provider_id, _| {
                let provider_id_str = provider_id.to_string();
                let prefix = get_provider_prefix(&provider_id_str);
                provider_id_str == provider_filter || prefix == provider_filter
            });
        }

        Ok(NrcData {
            providers_rewards,
            subnets_failure_rates,
        })
    }

    /// Collect underperforming nodes for a provider
    fn collect_underperforming_nodes(&self, daily_rewards: &[(DateUtc, DailyNodeProviderRewards)]) -> Vec<String> {
        let mut underperforming_nodes = Vec::new();
        for (_, rewards) in daily_rewards {
            for node_result in &rewards.daily_nodes_rewards {
                let multiplier = node_result.performance_multiplier.unwrap();
                if multiplier < 1.0 {
                    let node_id_str = node_result.node_id.unwrap().to_string();
                    let node_prefix = node_id_str.split('-').next().unwrap().to_string();
                    underperforming_nodes.push(node_prefix);
                }
            }
        }
        underperforming_nodes.sort();
        underperforming_nodes.dedup();
        underperforming_nodes
    }

    fn print_daily_summary_console(&self, nrc_data: &NrcData) -> anyhow::Result<()> {
        use tabled::settings::{Alignment, Merge, Modify, Style, Width, object::Rows};
        use tabled::{Table, Tabled};

        #[derive(Tabled)]
        struct DailyRewardSummary {
            #[tabled(rename = "Day")]
            day: String,
            #[tabled(rename = "Base Rewards Total")]
            base_rewards_total: String,
            #[tabled(rename = "Adjusted Rewards Total")]
            adjusted_rewards_total: String,
            #[tabled(rename = "Adjusted Rewards %")]
            adjusted_rewards_percent: String,
            #[tabled(rename = "Nodes")]
            nodes: usize,
            #[tabled(rename = "Assigned")]
            assigned_count: usize,
            #[tabled(rename = "Underperf")]
            underperf_count: usize,
            #[tabled(rename = "Underperf Nodes")]
            underperf_nodes: String,
        }

        println!("\n=== DAILY REWARDS SUMMARY ===");
        println!("Unit: XDRPermyriad per day");
        println!("\nLegend:");
        println!("• Day: UTC day (YYYY-MM-DD)");
        println!("• Base Rewards Total: Sum of base_rewards_xdr_permyriad across all nodes on that day");
        println!("• Adjusted Rewards Total: Sum of adjusted_rewards_xdr_permyriad across all nodes on that day");
        println!("• Adjusted Rewards %: (Adjusted Rewards Total / Base Rewards Total) × 100%");
        println!("• Nodes: Nodes found in registry on that day");
        println!("• Assigned: Nodes assigned to a subnet on that day");
        println!("• Underperf: Nodes with performance multiplier < 1 on that day");
        println!("• Underperf Nodes: Comma-separated underperforming node IDs (prefixes)");
        println!();

        for (provider_id, daily_rewards) in &nrc_data.providers_rewards {
            let provider_id_str = provider_id.to_string();
            let provider_prefix = get_provider_prefix(&provider_id_str);
            println!("\n=== Provider: {} ===", provider_prefix);

            let mut table_data = Vec::new();
            for (day, rewards) in daily_rewards {
                let day_str = Self::format_date_utc(*day);
                let nodes_in_registry = rewards.daily_nodes_rewards.len();
                let base_rewards_total: u64 = rewards.total_base_rewards_xdr_permyriad.unwrap();
                let adjusted_rewards_total: u64 = rewards.total_adjusted_rewards_xdr_permyriad.unwrap();

                // Calculate adjusted rewards percentage
                let adjusted_rewards_percent = if base_rewards_total > 0 {
                    format!("{:.2}%", (adjusted_rewards_total as f64 / base_rewards_total as f64) * 100.0)
                } else {
                    "N/A".to_string()
                };

                // Count assigned nodes
                let assigned_count = rewards
                    .daily_nodes_rewards
                    .iter()
                    .filter(|node_result| {
                        matches!(
                            node_result.daily_node_failure_rate,
                            Some(ic_node_rewards_canister_api::provider_rewards_calculation::DailyNodeFailureRate::SubnetMember { .. })
                        )
                    })
                    .count();

                let underperf_prefixes = self.collect_underperforming_nodes(&[(*day, rewards.clone())]);
                let underperforming_nodes_count = underperf_prefixes.len();
                let underperforming_nodes = if underperf_prefixes.is_empty() {
                    "None".to_string()
                } else {
                    let nodes_str = underperf_prefixes.join(", ");
                    if nodes_str.len() > 30 {
                        format!("{}...", &nodes_str[..27])
                    } else {
                        nodes_str
                    }
                };

                table_data.push(DailyRewardSummary {
                    day: day_str,
                    base_rewards_total: base_rewards_total.to_string(),
                    adjusted_rewards_total: adjusted_rewards_total.to_string(),
                    adjusted_rewards_percent,
                    nodes: nodes_in_registry,
                    assigned_count,
                    underperf_count: underperforming_nodes_count,
                    underperf_nodes: underperforming_nodes,
                });
            }

            let mut table = Table::new(table_data);
            table
                .with(Style::ascii())
                .with(Modify::new(Rows::new(0..1)).with(Alignment::center()))
                .with(Modify::new(Rows::new(1..)).with(Alignment::left()))
                .with(Width::wrap(250).keep_words(true))
                .with(Merge::vertical());

            println!("{}", table);
        }
        Ok(())
    }

    /// Display the comparison table
    fn print_comparison_console(
        &self,
        nrc_data: &NrcData,
        compare_with_governance: bool,
        governance_providers_rewards: &BTreeMap<PrincipalId, u64>,
    ) -> anyhow::Result<()> {
        use tabled::builder::Builder;
        use tabled::settings::object::{Columns, Rows};
        use tabled::settings::{Alignment, Merge, Modify, Style, Width};

        let mut builder = Builder::default();

        // Build header row dynamically
        let mut header = vec!["Provider", "Adjusted", "Base", "Adj-Base Diff", "Adj-Base %"];
        if compare_with_governance {
            header.extend_from_slice(&["Governance", "Adj-Gov Diff", "Adj-Gov %"]);
        }
        header.push("Underperf Nodes");
        builder.push_record(header);

        // Collect data for sorting
        let mut table_data: Vec<(f64, Vec<String>)> = Vec::new();

        for (provider_id, daily_rewards) in &nrc_data.providers_rewards {
            let provider_id_str = provider_id.to_string();
            let provider_prefix = get_provider_prefix(&provider_id_str);

            // Calculate adjusted rewards total
            let adjusted_total: u64 = daily_rewards
                .iter()
                .map(|(_, reward)| reward.total_adjusted_rewards_xdr_permyriad.unwrap())
                .sum();

            // Calculate base rewards total
            let base_total: u64 = daily_rewards
                .iter()
                .map(|(_, reward)| reward.total_base_rewards_xdr_permyriad.unwrap())
                .sum();

            // Calculate adj-base difference and percentage
            let adj_base_diff = adjusted_total as i64 - base_total as i64;
            let adj_base_percent = if base_total > 0 {
                adj_base_diff as f64 / base_total as f64 * 100.0
            } else {
                0.0
            };

            // Collect underperforming nodes
            let underperf_prefixes = self.collect_underperforming_nodes(daily_rewards);
            let underperforming_nodes_display = if underperf_prefixes.is_empty() {
                "None".to_string()
            } else {
                let nodes_str = underperf_prefixes.join(", ");
                if nodes_str.len() > 40 {
                    format!("{}...", &nodes_str[..37])
                } else {
                    nodes_str
                }
            };

            // Build row
            let mut row = vec![
                provider_prefix.to_string(),
                adjusted_total.to_string(),
                base_total.to_string(),
                adj_base_diff.to_string(),
                format!("{:.2}%", adj_base_percent),
            ];

            let sort_key = if compare_with_governance {
                // Get governance rewards and calculate difference
                let gov_total = governance_providers_rewards.get(provider_id).copied().unwrap_or(0);
                let adj_gov_diff = adjusted_total as i64 - gov_total as i64;
                let adj_gov_percent = if gov_total > 0 {
                    adj_gov_diff as f64 / gov_total as f64 * 100.0
                } else {
                    0.0
                };

                row.extend_from_slice(&[gov_total.to_string(), adj_gov_diff.to_string(), format!("{:.2}%", adj_gov_percent)]);

                adj_gov_percent.abs()
            } else {
                adj_base_percent.abs()
            };

            row.push(underperforming_nodes_display);
            table_data.push((sort_key, row));
        }

        // Sort by the appropriate percentage (descending)
        table_data.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Add sorted rows to builder
        for (_, row) in table_data {
            builder.push_record(row);
        }

        // Build and style the table
        let mut table = builder.build();

        let underperf_col_idx = if compare_with_governance { 8 } else { 5 };

        table
            .with(Style::modern())
            .with(Modify::new(Rows::new(0..1)).with(Alignment::center()))
            .with(Modify::new(Columns::new(underperf_col_idx..underperf_col_idx + 1)).with(Width::truncate(40).suffix("...")))
            .with(Width::wrap(250).keep_words(true))
            .with(Merge::vertical());

        // Print header and legend
        if compare_with_governance {
            println!("\n=== NODE REWARDS COMPARISON: ADJUSTED vs BASE vs GOVERNANCE ===");
            println!("Unit: XDRPermyriad | Sorted by decreasing |Adj-Gov %|");
            println!("\nLegend:");
            println!("• Adjusted: Adjusted rewards (performance-adjusted)");
            println!("• Base: Base rewards (without performance adjustment)");
            println!("• Adj-Base Diff: Adjusted - Base");
            println!("• Adj-Base %: (Adj-Base Diff / Base) × 100%");
            println!("• Governance: Governance distributed rewards");
            println!("• Adj-Gov Diff: Adjusted - Governance");
            println!("• Adj-Gov %: (Adj-Gov Diff / Governance) × 100%");
            println!("• Underperf Nodes: Comma-separated underperforming node IDs (prefixes)");
        } else {
            println!("\n=== NODE REWARDS COMPARISON: ADJUSTED vs BASE ===");
            println!("Unit: XDRPermyriad | Sorted by decreasing |Adj-Base %|");
            println!("\nLegend:");
            println!("• Adjusted: Adjusted rewards (performance-adjusted)");
            println!("• Base: Base rewards (without performance adjustment)");
            println!("• Adj-Base Diff: Adjusted - Base");
            println!("• Adj-Base %: (Adj-Base Diff / Base) × 100%");
            println!("• Underperf Nodes: Comma-separated underperforming node IDs (prefixes)");
        }
        println!();

        println!("{}", table);

        println!("\n=== SUMMARY ===");
        println!("Successfully processed {} providers", nrc_data.providers_rewards.len());

        Ok(())
    }

    /// Format DateUtc without the " UTC" suffix
    fn format_date_utc(date: DateUtc) -> String {
        let date_str = date.to_string();
        date_str.strip_suffix(" UTC").unwrap().to_string()
    }

    /// Generate CSV files split by provider
    async fn generate_csv_files_by_provider(&self, nrc_data: &NrcData, output_dir: &str) -> anyhow::Result<()> {
        // Create rewards directory with start_day_to_end_day format
        let start_day_str = self.ctx().start_date.format("%Y-%m-%d").to_string();
        let end_day_str = self.ctx().end_date.format("%Y-%m-%d").to_string();
        let dir_name = format!("rewards_{}_to_{}", start_day_str, end_day_str);
        let rewards_dir = format!("{}/{}", output_dir, dir_name);
        fs::create_dir_all(&rewards_dir)?;
        info!("Created rewards directory: {}", rewards_dir);

        // Generate CSV files for each provider separately
        for (provider_id, daily_rewards) in &nrc_data.providers_rewards {
            let provider_dir = format!("{}/{}", rewards_dir, provider_id);
            fs::create_dir_all(&provider_dir)?;

            self.create_base_rewards_csv(&provider_dir, daily_rewards)?;
            self.create_base_rewards_type3_csv(&provider_dir, daily_rewards)?;
            self.create_rewards_summary_csv(&provider_dir, daily_rewards)?;
            self.create_node_metrics_csv(&provider_dir, daily_rewards)?;
        }

        // Generate subnets failure rates CSV in the rewards directory
        self.create_subnets_failure_rates_csv(&rewards_dir, &nrc_data.subnets_failure_rates)?;

        Ok(())
    }

    /// Create base rewards CSV file
    fn create_base_rewards_csv(&self, output_dir: &str, daily_rewards: &[(DateUtc, DailyNodeProviderRewards)]) -> anyhow::Result<()> {
        let filename = format!("{}/base_rewards.csv", output_dir);

        let mut wtr = Writer::from_path(&filename).unwrap();
        wtr.write_record(["day_utc", "monthly_xdr_permyriad", "daily_xdr_permyriad", "node_reward_type", "region"])?;

        for (day, rewards) in daily_rewards {
            let day_str = Self::format_date_utc(*day);
            for base_reward in &rewards.base_rewards {
                wtr.write_record([
                    &day_str,
                    &base_reward.monthly_xdr_permyriad.unwrap().to_string(),
                    &base_reward.daily_xdr_permyriad.unwrap().to_string(),
                    base_reward.node_reward_type.as_ref().unwrap(),
                    base_reward.region.as_ref().unwrap(),
                ])
                .unwrap();
            }
        }

        wtr.flush().unwrap();
        Ok(())
    }

    /// Create base rewards type3 CSV file
    fn create_base_rewards_type3_csv(&self, output_dir: &str, daily_rewards: &[(DateUtc, DailyNodeProviderRewards)]) -> anyhow::Result<()> {
        let filename = format!("{}/base_rewards_type3.csv", output_dir);

        let mut wtr = Writer::from_path(&filename).unwrap();
        wtr.write_record([
            "day_utc",
            "region",
            "nodes_count",
            "avg_rewards_xdr_permyriad",
            "avg_coefficient",
            "daily_xdr_permyriad",
        ])?;

        for (day, rewards) in daily_rewards {
            let day_str = Self::format_date_utc(*day);
            for base_reward_type3 in &rewards.base_rewards_type3 {
                wtr.write_record([
                    &day_str,
                    base_reward_type3.region.as_ref().unwrap(),
                    &base_reward_type3.nodes_count.unwrap().to_string(),
                    &base_reward_type3.avg_rewards_xdr_permyriad.unwrap().to_string(),
                    &base_reward_type3.avg_coefficient.unwrap().to_string(),
                    &base_reward_type3.daily_xdr_permyriad.unwrap().to_string(),
                ])?;
            }
        }

        wtr.flush()?;
        Ok(())
    }

    /// Create rewards summary CSV file
    fn create_rewards_summary_csv(&self, output_dir: &str, daily_rewards: &[(DateUtc, DailyNodeProviderRewards)]) -> anyhow::Result<()> {
        let filename = format!("{}/rewards_summary.csv", output_dir);

        let mut wtr = Writer::from_path(&filename).unwrap();
        wtr.write_record([
            "day_utc",
            "base_rewards_total",
            "adjusted_rewards_total",
            "adjusted_rewards_percent",
            "nodes_in_registry",
            "assigned_nodes",
            "underperforming_nodes_count",
            "underperforming_nodes",
        ])
        .unwrap();

        for (day, rewards) in daily_rewards {
            let day_str = Self::format_date_utc(*day);

            // Sum base and adjusted rewards across all nodes for the day
            let base_rewards_total: u64 = rewards.total_base_rewards_xdr_permyriad.unwrap();
            let adjusted_rewards_total: u64 = rewards.total_adjusted_rewards_xdr_permyriad.unwrap();

            // Calculate adjusted rewards percentage
            let adjusted_rewards_percent = if base_rewards_total > 0 {
                format!("{:.2}", (adjusted_rewards_total as f64 / base_rewards_total as f64) * 100.0)
            } else {
                "N/A".to_string()
            };

            let nodes_in_registry = rewards.daily_nodes_rewards.len();

            // Count assigned nodes
            let assigned_count = rewards
                .daily_nodes_rewards
                .iter()
                .filter(|node_result| matches!(node_result.daily_node_failure_rate, Some(DailyNodeFailureRate::SubnetMember { .. })))
                .count();

            let mut underperf_prefixes: Vec<String> = rewards
                .daily_nodes_rewards
                .iter()
                .filter(|node_result| node_result.performance_multiplier.unwrap() < 1.0)
                .map(|node_result| {
                    let node_id_str = node_result.node_id.unwrap().to_string();
                    node_id_str.split('-').next().unwrap().to_string()
                })
                .collect();
            underperf_prefixes.sort();
            underperf_prefixes.dedup();
            let underperforming_nodes_count = underperf_prefixes.len();
            let underperforming_nodes = underperf_prefixes.join(", ");

            wtr.write_record([
                &day_str,
                &base_rewards_total.to_string(),
                &adjusted_rewards_total.to_string(),
                &adjusted_rewards_percent,
                &nodes_in_registry.to_string(),
                &assigned_count.to_string(),
                &underperforming_nodes_count.to_string(),
                &underperforming_nodes,
            ])?;
        }

        wtr.flush()?;
        Ok(())
    }

    /// Create node metrics CSV files: by day, by node, and by performance_multiplier
    fn create_node_metrics_csv(&self, output_dir: &str, daily_rewards: &[(DateUtc, DailyNodeProviderRewards)]) -> anyhow::Result<()> {
        // Writers for the three views
        let filename_day = format!("{}/node_metrics_by_day.csv", output_dir);
        let filename_node = format!("{}/node_metrics_by_node.csv", output_dir);
        let filename_perf = format!("{}/node_metrics_by_performance_multiplier.csv", output_dir);
        let mut wtr_day = Writer::from_path(&filename_day)?;
        let mut wtr_node = Writer::from_path(&filename_node)?;
        let mut wtr_perf = Writer::from_path(&filename_perf)?;
        // Collector to group rows by node for the node-centric view
        let mut by_node: BTreeMap<String, Vec<Vec<String>>> = BTreeMap::new();
        // Collector for performance_multiplier sorted view
        let mut by_performance: Vec<(f64, DateUtc, Vec<String>)> = Vec::new();

        // Headers
        // By day: day first
        wtr_day
            .write_record([
                "day_utc",
                "node_id",
                "node_reward_type",
                "region",
                "dc_id",
                "node_status",
                "performance_multiplier",
                "base_rewards_xdr_permyriad",
                "adjusted_rewards_xdr_permyriad",
                "subnet_assigned",
                "subnet_assigned_failure_rate",
                "num_blocks_proposed",
                "num_blocks_failed",
                "original_failure_rate",
                "relative_failure_rate",
                "extrapolated_failure_rate",
            ])
            .unwrap();

        // By node: node first
        wtr_node
            .write_record([
                "node_id",
                "day_utc",
                "node_reward_type",
                "region",
                "dc_id",
                "node_status",
                "performance_multiplier",
                "base_rewards_xdr_permyriad",
                "adjusted_rewards_xdr_permyriad",
                "subnet_assigned",
                "subnet_assigned_failure_rate",
                "num_blocks_proposed",
                "num_blocks_failed",
                "original_failure_rate",
                "relative_failure_rate",
                "extrapolated_failure_rate",
            ])
            .unwrap();

        // By performance_multiplier: same header as by-day
        wtr_perf
            .write_record([
                "day_utc",
                "node_id",
                "node_reward_type",
                "region",
                "dc_id",
                "node_status",
                "performance_multiplier",
                "base_rewards_xdr_permyriad",
                "adjusted_rewards_xdr_permyriad",
                "subnet_assigned",
                "subnet_assigned_failure_rate",
                "num_blocks_proposed",
                "num_blocks_failed",
                "original_failure_rate",
                "relative_failure_rate",
                "extrapolated_failure_rate",
            ])
            .unwrap();

        for (day, rewards) in daily_rewards {
            let day_str = Self::format_date_utc(*day);
            for node_result in &rewards.daily_nodes_rewards {
                // Flatten DailyNodeFailureRate/NodeMetricsDaily
                let (
                    status_str,
                    subnet_assigned,
                    subnet_assigned_fr,
                    num_blocks_proposed,
                    num_blocks_failed,
                    original_fr,
                    relative_fr,
                    extrapolated_fr,
                ) = match &node_result.daily_node_failure_rate {
                    Some(DailyNodeFailureRate::SubnetMember { node_metrics: Some(m) }) => (
                        "Assigned".to_string(),
                        m.subnet_assigned.map(|s| s.to_string()).unwrap_or_default(),
                        m.subnet_assigned_failure_rate.map(|f| f.to_string()).unwrap_or_default(),
                        m.num_blocks_proposed.map(|n| n.to_string()).unwrap_or_default(),
                        m.num_blocks_failed.map(|n| n.to_string()).unwrap_or_default(),
                        m.original_failure_rate.map(|f| f.to_string()).unwrap_or_default(),
                        m.relative_failure_rate.map(|f| f.to_string()).unwrap_or_default(),
                        String::new(),
                    ),
                    Some(DailyNodeFailureRate::NonSubnetMember { extrapolated_failure_rate }) => (
                        "Unassigned".to_string(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        extrapolated_failure_rate.map(|f| f.to_string()).unwrap_or_default(),
                    ),
                    _ => panic!("This should never happen"),
                };

                // Row for by-day
                wtr_day
                    .write_record([
                        &day_str,
                        &node_result.node_id.map(|id| id.to_string()).unwrap_or_default(),
                        node_result.node_reward_type.as_ref().unwrap(),
                        node_result.region.as_ref().unwrap(),
                        node_result.dc_id.as_ref().unwrap(),
                        &status_str,
                        &node_result.performance_multiplier.unwrap().to_string(),
                        &node_result.base_rewards_xdr_permyriad.unwrap().to_string(),
                        &node_result.adjusted_rewards_xdr_permyriad.unwrap().to_string(),
                        &subnet_assigned,
                        &subnet_assigned_fr,
                        &num_blocks_proposed,
                        &num_blocks_failed,
                        &original_fr,
                        &relative_fr,
                        &extrapolated_fr,
                    ])
                    .unwrap();

                // Collect row for by-node (omit node_id here, we'll prepend it when writing)
                by_node
                    .entry(node_result.node_id.map(|id| id.to_string()).unwrap_or_default())
                    .or_default()
                    .push(vec![
                        day_str.clone(),
                        node_result.node_reward_type.as_ref().unwrap().to_string(),
                        node_result.region.as_ref().unwrap().to_string(),
                        node_result.dc_id.as_ref().unwrap().to_string(),
                        status_str.clone(),
                        node_result.performance_multiplier.unwrap().to_string(),
                        node_result.base_rewards_xdr_permyriad.unwrap().to_string(),
                        node_result.adjusted_rewards_xdr_permyriad.unwrap().to_string(),
                        subnet_assigned.clone(),
                        subnet_assigned_fr.clone(),
                        num_blocks_proposed.clone(),
                        num_blocks_failed.clone(),
                        original_fr.clone(),
                        relative_fr.clone(),
                        extrapolated_fr.clone(),
                    ]);

                // Collect row for by-performance_multiplier (same format as by-day)
                let performance_multiplier = node_result.performance_multiplier.unwrap();
                let perf_row = vec![
                    day_str.clone(),
                    node_result.node_id.map(|id| id.to_string()).unwrap_or_default(),
                    node_result.node_reward_type.as_ref().unwrap().to_string(),
                    node_result.region.as_ref().unwrap().to_string(),
                    node_result.dc_id.as_ref().unwrap().to_string(),
                    status_str.clone(),
                    performance_multiplier.to_string(),
                    node_result.base_rewards_xdr_permyriad.unwrap().to_string(),
                    node_result.adjusted_rewards_xdr_permyriad.unwrap().to_string(),
                    subnet_assigned.clone(),
                    subnet_assigned_fr.clone(),
                    num_blocks_proposed.clone(),
                    num_blocks_failed.clone(),
                    original_fr.clone(),
                    relative_fr.clone(),
                    extrapolated_fr.clone(),
                ];
                by_performance.push((performance_multiplier, *day, perf_row));
            }
        }

        // Write grouped-by-node rows: each node contiguous
        for (node_id, rows) in by_node {
            for row in rows {
                let mut full = Vec::with_capacity(1 + row.len());
                full.push(node_id.clone());
                full.extend(row);
                wtr_node.write_record(full).unwrap();
            }
        }

        // Sort by performance_multiplier ascending, then by day
        by_performance.sort_by(|a, b| {
            let perf_cmp = a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal);
            if perf_cmp == std::cmp::Ordering::Equal {
                a.1.cmp(&b.1)
            } else {
                perf_cmp
            }
        });

        // Write sorted-by-performance rows
        for (_, _, row) in by_performance {
            wtr_perf.write_record(row)?;
        }

        wtr_day.flush()?;
        wtr_node.flush()?;
        wtr_perf.flush()?;
        Ok(())
    }

    /// Create subnets failure rates CSV file
    fn create_subnets_failure_rates_csv(&self, output_dir: &str, subnets_fr_data: &BTreeMap<SubnetId, Vec<(DateUtc, f64)>>) -> anyhow::Result<()> {
        let filename = format!("{}/subnets_failure_rates.csv", output_dir);
        let mut wtr = Writer::from_path(&filename).unwrap();

        wtr.write_record(["subnet_id", "day_utc", "failure_rate"])?;

        // Sort by subnet_id first, then by day_utc
        subnets_fr_data
            .iter()
            .flat_map(|(subnet_id, subnets_fr)| subnets_fr.iter().map(|(date, fr)| (*subnet_id, Self::format_date_utc(*date), *fr)))
            .sorted_by(|a, b| {
                let subnet_cmp = a.1.cmp(&b.1);
                if subnet_cmp == std::cmp::Ordering::Equal {
                    a.0.cmp(&b.0)
                } else {
                    subnet_cmp
                }
            })
            .map(|(subnet_id, date, fr)| (subnet_id.to_string(), date, fr.to_string()))
            .for_each(|(subnet_id, date, fr)| {
                wtr.write_record([subnet_id, date, fr]).unwrap();
            });

        wtr.flush()?;
        Ok(())
    }

    async fn push_rewards_metrics_to_victoria(&self, ctx: crate::ctx::DreContext, date: Option<String>, victoria_url: Url) -> anyhow::Result<()> {
        use prometheus::{GaugeVec, Opts, Registry};
        // Parse or default to yesterday
        let target_date = if let Some(date) = date {
            chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")?
        } else {
            let today = chrono::Utc::now().date_naive();
            today.pred_opt().unwrap()
        };

        info!("Pushing node rewards data to VictoriaMetrics for date: {}", target_date);

        // Fetch data from node rewards canister
        let (_, canister_agent) = ctx.create_ic_agent_canister_client().await?;
        let node_rewards_client: NodeRewardsCanisterWrapper = canister_agent.into();

        let date_utc = DateUtc::from(target_date);
        let daily_results = node_rewards_client.get_rewards_daily(date_utc).await?;

        // Create Prometheus registry and metrics
        let registry = Registry::new();

        let noon_timestamp_millis = target_date
            .and_hms_opt(12, 0, 0)
            .ok_or_else(|| anyhow!("Invalid date noon"))?
            .and_utc()
            .timestamp_millis();

        // Define metrics
        let nodes_count = GaugeVec::new(Opts::new("nodes_count", "Number of nodes per provider"), &["provider_id"])?;
        registry.register(Box::new(nodes_count.clone()))?;

        let base_rewards = GaugeVec::new(
            Opts::new("total_base_rewards_xdr_permyriad", "Total base rewards in XDR permyriad"),
            &["provider_id"],
        )?;
        registry.register(Box::new(base_rewards.clone()))?;

        let adjusted_rewards = GaugeVec::new(
            Opts::new("total_adjusted_rewards_xdr_permyriad", "Total adjusted rewards in XDR permyriad"),
            &["provider_id"],
        )?;
        registry.register(Box::new(adjusted_rewards.clone()))?;

        let original_failure_rate = GaugeVec::new(
            Opts::new("original_failure_rate", "Original failure rate per node"),
            &["provider_id", "node_id", "subnet_id"],
        )?;
        registry.register(Box::new(original_failure_rate.clone()))?;

        let relative_failure_rate = GaugeVec::new(
            Opts::new("relative_failure_rate", "Relative failure rate per node"),
            &["provider_id", "node_id", "subnet_id"],
        )?;
        registry.register(Box::new(relative_failure_rate.clone()))?;

        let subnet_failure = GaugeVec::new(Opts::new("subnet_failure_rate", "Failure rate per subnet"), &["subnet_id"])?;
        registry.register(Box::new(subnet_failure.clone()))?;

        let gov_timestamp = prometheus::Gauge::new(
            "governance_latest_reward_event_timestamp_seconds",
            "Latest governance reward event timestamp",
        )?;
        registry.register(Box::new(gov_timestamp.clone()))?;

        // Set provider-level metrics
        for (provider_id, provider_rewards) in daily_results.provider_results {
            let provider_id_str = provider_id.to_string();

            nodes_count
                .with_label_values(&[&provider_id_str])
                .set(provider_rewards.daily_nodes_rewards.len() as f64);

            if let Some(base) = provider_rewards.total_base_rewards_xdr_permyriad {
                base_rewards.with_label_values(&[&provider_id_str]).set(base as f64);
            }

            if let Some(adjusted) = provider_rewards.total_adjusted_rewards_xdr_permyriad {
                adjusted_rewards.with_label_values(&[&provider_id_str]).set(adjusted as f64);
            }

            // Node-level metrics
            for node_result in &provider_rewards.daily_nodes_rewards {
                let node_id_str = node_result.node_id.map(|id| id.to_string()).unwrap_or_default();

                if let Some(DailyNodeFailureRate::SubnetMember { node_metrics: Some(m) }) = &node_result.daily_node_failure_rate {
                    let subnet_id_str = m.subnet_assigned.map(|s| s.to_string()).unwrap_or_default();

                    if let Some(original_fr) = m.original_failure_rate {
                        original_failure_rate
                            .with_label_values(&[&provider_id_str, &node_id_str, &subnet_id_str])
                            .set(original_fr);
                    }

                    if let Some(relative_fr) = m.relative_failure_rate {
                        relative_failure_rate
                            .with_label_values(&[&provider_id_str, &node_id_str, &subnet_id_str])
                            .set(relative_fr);
                    }
                }
            }
        }

        // Set subnet-level metrics
        for (subnet_id, failure_rate) in daily_results.subnets_failure_rate {
            subnet_failure.with_label_values(&[&subnet_id.to_string()]).set(failure_rate);
        }

        // Set governance timestamp
        let governance_client: GovernanceCanisterWrapper = ctx.create_ic_agent_canister_client().await?.1.into();
        let gov_rewards_list = governance_client.list_node_provider_rewards(None).await?;
        if let Some(last_rewards) = gov_rewards_list.first() {
            gov_timestamp.set(last_rewards.timestamp as f64);
        }

        // Gather metrics and manually format with timestamps
        let metric_families = registry.gather();
        let mut buffer = String::new();

        // Manually construct Prometheus format with timestamps
        for mf in &metric_families {
            for m in mf.get_metric() {
                let metric_name = mf.get_name();

                // Build label string
                let mut labels = Vec::new();
                for label in m.get_label() {
                    labels.push(format!("{}=\"{}\"", label.get_name(), label.get_value()));
                }
                let label_str = if labels.is_empty() {
                    String::new()
                } else {
                    format!("{{{}}}", labels.join(","))
                };

                // Get metric value
                let value = if m.has_gauge() {
                    m.get_gauge().get_value()
                } else if m.has_counter() {
                    m.get_counter().get_value()
                } else if m.has_untyped() {
                    m.get_untyped().get_value()
                } else {
                    continue; // Skip unsupported metric types
                };

                // Format: metric_name{labels} value timestamp_ms
                buffer.push_str(&format!("{}{} {} {}\n", metric_name, label_str, value, noon_timestamp_millis));
            }
        }

        // Push to VictoriaMetrics via import API
        let client = reqwest::Client::new();
        let import_url = victoria_url
            .join("/api/v1/import/prometheus")
            .map_err(|e| anyhow!("Failed to construct VictoriaMetrics import URL: {}", e))?;
        let response = client.post(import_url).header("Content-Type", "text/plain").body(buffer).send().await?;

        if response.status().is_success() {
            info!("Successfully pushed node rewards data for {} to VictoriaMetrics", target_date);
            Ok(())
        } else {
            let error_text = response.text().await?;
            Err(anyhow!("Failed to push to VictoriaMetrics: {}", error_text))
        }
    }
}
