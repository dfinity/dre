use crate::commands::node_rewards::common::CsvGenerator;
use crate::{auth::AuthRequirement, exe::args::GlobalArgs, exe::ExecutableCommand};
use chrono::Datelike;
use chrono::{DateTime, NaiveDate};
use clap::{Args, Subcommand};
use csv::Writer;
use futures_util::future::join_all;
use ic_base_types::{PrincipalId, SubnetId};
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_canisters::node_rewards::NodeRewardsCanisterWrapper;
use ic_node_rewards_canister_api::provider_rewards_calculation::{DailyNodeFailureRate, DailyNodeProviderRewards, DailyResults};
use ic_node_rewards_canister_api::DateUtc;
use itertools::Itertools;
use log::info;
use std::collections::BTreeMap;
use std::fs;
use tabled::{
    settings::{object::Rows, Alignment, Merge, Modify, Style, Width}, Table,
    Tabled,
};
use crate::commands::node_rewards::NodeRewardsMode::Ongoing;
use crate::commands::node_rewards::ongoing::OngoingRewardsCommand;
use crate::commands::node_rewards::past_rewards::PastRewardsCommand;

mod common;
mod ongoing;
mod past_rewards;

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
    },
}

#[derive(Args, Debug)]
pub struct NodeRewards {
    /// Subcommand mode: ongoing or past-rewards <month>
    #[command(subcommand)]
    pub mode: NodeRewardsMode,
}

impl ExecutableCommand for NodeRewards {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Signer
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let (_, canister_agent) = ctx.create_ic_agent_canister_client().await?;
        info!("Started action...");

        let node_rewards_client: NodeRewardsCanisterWrapper = canister_agent.clone().into();
        let governance_client: GovernanceCanisterWrapper = canister_agent.into();

        let mut gov_rewards_list = governance_client.list_node_provider_rewards(None).await?;
        // Run the selected subcommand
        let (start_date, end_date, maybe_provider_filter, maybe_csv_output_path, maybe_gov_rewards_for_comparison) = match &self.mode {
            NodeRewardsMode::Ongoing {
                csv_detailed_output_path,
                provider_id,
            } => OngoingRewardsCommand::run(canister_agent, csv_detailed_output_path, provider_id)
            NodeRewardsMode::PastRewards {
                month,
                csv_detailed_output_path,
                provider_id
            } => PastRewardsCommand::run(canister_agent, csv_detailed_output_path, provider_id, month)
        };

        Ok(())
    }
}
