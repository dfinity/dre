use std::fs;

use crate::{auth::AuthRequirement, exe::args::GlobalArgs, exe::ExecutableCommand};
use clap::Args;
use ic_canisters::node_rewards::NodeRewardsCanisterWrapper;
use ic_types::PrincipalId;
use log::info;
use rewards_calculation::types::DayUtc;
use ic_canisters::governance::GovernanceCanisterWrapper;
use super::csv_trait::CsvGenerator;

#[derive(Args, Debug)]
pub struct Ongoing {
    /// Provider ID to fetch rewards for
    #[arg(long)]
    pub provider_id: PrincipalId,

    /// Optional path to save CSV output. If not provided, displays table in console.
    #[arg(long)]
    pub to_csv: Option<String>,

    /// Display results in XDRPermyriad instead of ICP
    #[arg(long)]
    pub xdr_permyriad: bool,
}

impl Ongoing {
    /// Get ongoing rewards for a specific provider
    async fn get_provider_ongoing_rewards(
        &self,
        canister_agent: ic_canisters::IcAgentCanisterClient,
        provider_id: PrincipalId,
        to_csv: Option<String>,
        xdr_permyriad: bool,
    ) -> anyhow::Result<()> {
        let node_rewards_client: NodeRewardsCanisterWrapper = canister_agent.clone().into();
        let governance_client: GovernanceCanisterWrapper = canister_agent.into();
        let mut gov_rewards = governance_client.list_node_provider_rewards(None).await?.into_iter();
        let last_rewards = gov_rewards.next().unwrap();

        let current_timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        let end_day = DayUtc::from_secs(current_timestamp).previous_day();
        let start_day = DayUtc::from_secs(last_rewards.timestamp);
        let daily_rewards = node_rewards_client
            .get_provider_rewards_daily(provider_id, start_day, end_day)
            .await?;

        if daily_rewards.is_empty() {
            println!("No rewards data found for provider {}", provider_id);
            return Ok(());
        }

        // Convert to CSV format
        let csv_data = self.convert_daily_rewards_to_csv(&daily_rewards, provider_id, xdr_permyriad).await?;

        if let Some(csv_path) = to_csv {
            // Write to CSV file
            fs::write(&csv_path, &csv_data)?;
            println!("CSV data written to: {}", csv_path);
        } else {
            // Display as table using tabled
            let lines: Vec<&str> = csv_data.lines().collect();
            if lines.len() > 1 {
                for (i, line) in lines.iter().enumerate() {
                    if i == 0 {
                        // Skip header for now, just print it
                        println!("\n=== ONGOING REWARDS FOR PROVIDER {} ===", provider_id);
                        println!(
                            "Unit: {} | Days: {} to {}",
                            if xdr_permyriad { "XDRPermyriad" } else { "ICP" },
                            start_day.unix_ts_at_day_start_nanoseconds(),
                            end_day.unix_ts_at_day_start_nanoseconds()
                        );
                        println!("\n{}", line);
                        println!("{}", "=".repeat(line.len()));
                    } else {
                        let fields: Vec<&str> = line.split(',').collect();
                        if fields.len() >= 6 {
                            println!(
                                "Day: {} | Provider: {} | Rewards: {} | Nodes: {} | Avg Rewards: {} | Avg Coeff: {}%",
                                fields[0], fields[1], fields[2], fields[3], fields[4], fields[5]
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl CsvGenerator for Ongoing {
    // All methods are now provided by the trait's default implementations
}

impl ExecutableCommand for Ongoing {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Signer
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let (_, canister_agent) = ctx.create_ic_agent_canister_client().await.unwrap();
        info!("Started action...");

        self.get_provider_ongoing_rewards(canister_agent, self.provider_id, self.to_csv.clone(), self.xdr_permyriad)
            .await
            .unwrap();

        Ok(())
    }
}
