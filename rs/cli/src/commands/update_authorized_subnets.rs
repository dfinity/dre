use ic_canisters::cycles_minting::CyclesMintingCanisterWrapper;
use indexmap::IndexMap;
use std::{path::PathBuf, sync::Arc};

use clap::{error::ErrorKind, Args};
use ic_management_types::Subnet;
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use itertools::Itertools;
use log::info;

use crate::{
    forum::{ic_admin::forum_enabled_proposer, ForumParameters, ForumPostKind},
    ic_admin::{ProposeCommand, ProposeOptions},
};

use super::{AuthRequirement, ExecutableCommand};

const DEFAULT_CANISTER_LIMIT: u64 = 60_000;
const DEFAULT_STATE_SIZE_BYTES_LIMIT: u64 = 400 * 1024 * 1024 * 1024; // 400GB

const DEFAULT_AUTHORIZED_SUBNETS_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/non_public_subnets.csv"));

#[derive(Args, Debug)]
pub struct UpdateAuthorizedSubnets {
    /// Path to csv file containing the blacklist.
    #[clap(long)]
    path: Option<PathBuf>,

    /// Canister num limit for marking a subnet as non public
    #[clap(default_value_t = DEFAULT_CANISTER_LIMIT)]
    canister_limit: u64,

    /// Size limit for marking a subnet as non public in bytes
    #[clap(default_value_t = DEFAULT_STATE_SIZE_BYTES_LIMIT)]
    state_size_limit: u64,

    /// Number of verified subnets to open that weren't open before
    #[clap(long, default_value_t = 1)]
    open_verified_subnets: i32,

    #[clap(flatten)]
    pub forum_parameters: ForumParameters,
}

impl ExecutableCommand for UpdateAuthorizedSubnets {
    fn require_auth(&self) -> AuthRequirement {
        super::AuthRequirement::Neuron
    }

    fn validate(&self, _args: &crate::commands::Args, cmd: &mut clap::Command) {
        if let Some(path) = &self.path {
            if !path.exists() {
                cmd.error(ErrorKind::InvalidValue, format!("Path `{}` not found", path.display())).exit()
            }

            if !path.is_file() {
                cmd.error(ErrorKind::InvalidValue, format!("Path `{}` found, but is not a file", path.display()))
                    .exit()
            }
        }
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let non_public_subnets_csv = self.parse_csv()?;
        info!("Found following elements: {:?}", non_public_subnets_csv);

        let registry = ctx.registry().await;
        let subnets = registry.subnets().await?;
        let mut excluded_subnets = IndexMap::new();

        let human_bytes = human_bytes::human_bytes(self.state_size_limit as f64);
        let (_, agent) = ctx.create_ic_agent_canister_client().await?;

        let cmc = CyclesMintingCanisterWrapper::from(agent.clone());
        let public_subnets = cmc.get_authorized_subnets().await?;

        let mut verified_subnets_to_open = self.open_verified_subnets;

        for subnet in subnets.values().sorted_by_cached_key(|s| s.principal) {
            if subnet.subnet_type.eq(&SubnetType::System) {
                excluded_subnets.insert(subnet.principal, "System subnets should not have public access".to_string());
                continue;
            }

            // Check if subnet is explicitly marked as non-public
            let subnet_principal_string = subnet.principal.to_string();
            if let Some((_, description)) = non_public_subnets_csv
                .iter()
                .find(|(short_id, _)| subnet_principal_string.starts_with(short_id))
            {
                excluded_subnets.insert(subnet.principal, format!("Explicitly marked as non-public ({})", description));
                continue;
            }

            // Check if subnet utilization metrics are too high
            let subnet_metrics = agent.read_state_subnet_metrics(&subnet.principal).await?;

            if subnet_metrics.num_canisters >= self.canister_limit {
                excluded_subnets.insert(subnet.principal, format!("Subnet has more than {} canisters", self.canister_limit));
                continue;
            }

            if subnet_metrics.canister_state_bytes >= self.state_size_limit {
                excluded_subnets.insert(subnet.principal, format!("Subnet has more than {} state size", human_bytes));
            }

            // There was a request to open up 1 verified subnet per week
            if subnet.subnet_type.eq(&SubnetType::VerifiedApplication) {
                if public_subnets.contains(&subnet.principal) {
                    continue;
                }
                // A sufficient number of verified_subnets has already been opened up
                if verified_subnets_to_open == 0 {
                    // Check if the subnet ID matches any entry in the CSV and use the description.
                    // If no match is found, default to a generic message.
                    let description = non_public_subnets_csv
                        .iter()
                        .find(|(short_id, _)| subnet.principal.to_string().starts_with(short_id))
                        .map(|(_, desc)| desc.to_string())
                        .unwrap_or("Other verified subnets opened up in this run".to_string());
                    excluded_subnets.insert(subnet.principal, description);
                    continue;
                }
            }

            // Looks like we're good to go!
            // Now only adjust the counter of how many VerifiedApplication subnets have been opened
            // up in this run.
            if subnet.subnet_type.eq(&SubnetType::VerifiedApplication) && !public_subnets.contains(&subnet.principal) {
                if verified_subnets_to_open > 0 {
                    verified_subnets_to_open -= 1;
                } else {
                    unreachable!(
                        "Error in logic! Subnet of type VerifiedApplication cannot be let through the filter if `open_verified_subnets` is 0"
                    )
                }
            }
        }

        let summary = construct_summary(&subnets, &excluded_subnets, public_subnets)?;

        let authorized = subnets
            .keys()
            .filter(|subnet_id| !excluded_subnets.contains_key(*subnet_id))
            .cloned()
            .collect();

        let ic_admin = ctx.ic_admin().await?;

        let cmd = ProposeCommand::SetAuthorizedSubnetworks { subnets: authorized };
        let opts = ProposeOptions {
            title: Some("Updating the list of public subnets".to_string()),
            summary: Some(summary.clone()),
            motivation: None,
            forum_post_link: Some("[comment]: <> (Link will be added on actual execution)".to_string()),
        };

        forum_enabled_proposer(&self.forum_parameters, &ctx, ic_admin)
            .propose_with_possible_confirmation(cmd, opts, ForumPostKind::AuthorizedSubnetsUpdate { body: summary })
            .await
    }
}

impl UpdateAuthorizedSubnets {
    fn parse_csv(&self) -> anyhow::Result<Vec<(String, String)>> {
        let contents = match &self.path {
            Some(p) => fs_err::read_to_string(p)?,
            None => {
                info!("Using embedded version of authorized subnets csv that is added during build time");
                DEFAULT_AUTHORIZED_SUBNETS_CSV.to_string()
            }
        };
        let mut ret = vec![];
        for line in contents.lines() {
            if line.starts_with("subnet id") {
                info!("Skipping header line in csv");
                continue;
            }

            let (id, desc) = line.split_once(',').ok_or(anyhow::anyhow!("Failed to parse line: {}", line))?;
            ret.push((id.to_string(), desc.to_string()))
        }

        Ok(ret)
    }
}

/// FIXME probably should be moved to the Discourse post creation code.
/// also it would be wise to divorce the Discourse machinery from the kind of post
/// we want to compose, so that composing the post and posting the post are separate activities,
/// which they are.
fn construct_summary(
    subnets: &Arc<IndexMap<PrincipalId, Subnet>>,
    excluded_subnets: &IndexMap<PrincipalId, String>,
    current_public_subnets: Vec<PrincipalId>,
) -> anyhow::Result<String> {
    Ok(format!(
        "Updating the list of authorized subnets to:

| Subnet id | Subnet Type | Public | Description |
| --------- | ----------- | ------ | ----------- |
{}
",
        subnets
            .values()
            .map(|s| {
                let excluded_desc = excluded_subnets.get(&s.principal);
                let was_public = current_public_subnets.iter().any(|principal| principal == &s.principal);
                format!(
                    "| {} | {} | {} | {} |",
                    s.principal,
                    match &s.subnet_type {
                        SubnetType::Application => "Application",
                        SubnetType::System => "System",
                        SubnetType::VerifiedApplication => "Verified Application",
                    },
                    match (was_public, excluded_desc.is_none()) {
                        // The state doesn't change
                        (was_public, is_excluded) if was_public == is_excluded => was_public.to_string(),
                        // It changed from `was_public` to `is_excluded`
                        (was_public, is_excluded) => format!("~~{}~~ ⇒ {}", was_public, is_excluded),
                    },
                    excluded_desc.map(|s| s.to_string()).unwrap_or_default()
                )
            })
            .join("\n")
    ))
}
