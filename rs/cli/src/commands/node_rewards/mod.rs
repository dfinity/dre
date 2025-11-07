use crate::commands::node_rewards::ongoing::OngoingRewardsCommand;
use crate::commands::node_rewards::past_rewards::PastRewardsCommand;
use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};
use clap::{Args, Subcommand};
use log::info;

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

        // Run the selected subcommand
        match &self.mode {
            NodeRewardsMode::Ongoing {
                csv_detailed_output_path,
                provider_id,
            } => OngoingRewardsCommand::run(canister_agent.clone(), self, csv_detailed_output_path.as_deref(), provider_id.as_deref()).await,
            NodeRewardsMode::PastRewards {
                month,
                csv_detailed_output_path,
                provider_id,
            } => PastRewardsCommand::run(canister_agent.clone(), month, csv_detailed_output_path.as_deref(), provider_id.as_deref()).await,
        }
    }
}
