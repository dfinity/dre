use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    sync::Arc,
};

use clap::{error::ErrorKind, Args};
use ic_management_types::Subnet;
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use itertools::Itertools;
use log::info;

use crate::ic_admin::{ProposeCommand, ProposeOptions};

use super::ExecutableCommand;

const DEFAULT_CANISTER_LIMIT: u64 = 60_000;
const DEFAULT_STATE_SIZE_BYTES_LIMIT: u64 = 322_122_547_200; // 300GB

#[derive(Args, Debug)]
pub struct UpdateAuthorizedSubnets {
    /// Path to csv file containing the blacklist.
    #[clap(default_value = "./facts-db/non_public_subnets.csv")]
    path: PathBuf,

    /// Canister num limit for marking a subnet as non public
    #[clap(default_value_t = DEFAULT_CANISTER_LIMIT)]
    canister_limit: u64,

    /// Size limit for marking a subnet as non public in bytes
    #[clap(default_value_t = DEFAULT_STATE_SIZE_BYTES_LIMIT)]
    state_size_limit: u64,
}

impl ExecutableCommand for UpdateAuthorizedSubnets {
    fn require_ic_admin(&self) -> super::IcAdminRequirement {
        super::IcAdminRequirement::Detect
    }

    fn validate(&self, cmd: &mut clap::Command) {
        if !self.path.exists() {
            cmd.error(ErrorKind::InvalidValue, format!("Path `{}` not found", self.path.display()))
                .exit();
        }

        if !self.path.is_file() {
            cmd.error(
                ErrorKind::InvalidValue,
                format!("Path `{}` found, but is not a file", self.path.display()),
            );
        }
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let csv_contents = self.parse_csv()?;
        info!("Found following elements: {:?}", csv_contents);

        let registry = ctx.registry().await;
        let subnets = registry.subnets().await?;
        let mut excluded_subnets = BTreeMap::new();

        let human_bytes = human_bytes::human_bytes(self.state_size_limit as f64);
        let agent = ctx.create_ic_agent_canister_client(None)?;

        for subnet in subnets.values() {
            if subnet.subnet_type.eq(&SubnetType::System) {
                excluded_subnets.insert(subnet.principal, "System subnet".to_string());
                continue;
            }

            let subnet_principal_string = subnet.principal.to_string();
            if let Some((_, description)) = csv_contents.iter().find(|(short_id, _)| subnet_principal_string.starts_with(short_id)) {
                excluded_subnets.insert(subnet.principal, description.to_owned());
                continue;
            }

            let subnet_metrics = agent.read_state_subnet_metrics(&subnet.principal).await?;

            if subnet_metrics.num_canisters >= self.canister_limit {
                excluded_subnets.insert(subnet.principal, format!("Subnet has more than {} canisters", self.canister_limit));
                continue;
            }

            if subnet_metrics.canister_state_bytes >= self.state_size_limit {
                excluded_subnets.insert(subnet.principal, format!("Subnet has more than {} state size", human_bytes));
            }
        }

        let summary = construct_summary(&subnets, &excluded_subnets)?;

        let authorized = subnets
            .keys()
            .filter(|subnet_id| !excluded_subnets.contains_key(subnet_id))
            .cloned()
            .collect();

        let ic_admin = ctx.ic_admin();
        ic_admin
            .propose_run(
                ProposeCommand::SetAuthorizedSubnetworks { subnets: authorized },
                ProposeOptions {
                    title: Some("Update list of public subnets".to_string()),
                    summary: Some(summary),
                    motivation: None,
                },
            )
            .await?;

        Ok(())
    }
}

impl UpdateAuthorizedSubnets {
    fn parse_csv(&self) -> anyhow::Result<Vec<(String, String)>> {
        let contents = BufReader::new(File::open(&self.path)?);
        let mut ret = vec![];
        for line in contents.lines() {
            let content = line?;
            if content.starts_with("subnet id") {
                info!("Skipping header line in csv");
                continue;
            }

            let (id, desc) = content.split_once(',').ok_or(anyhow::anyhow!("Failed to parse line: {}", content))?;
            ret.push((id.to_string(), desc.to_string()))
        }

        Ok(ret)
    }
}

fn construct_summary(subnets: &Arc<BTreeMap<PrincipalId, Subnet>>, excluded_subnets: &BTreeMap<PrincipalId, String>) -> anyhow::Result<String> {
    Ok(format!(
        "Updating the list of authorized subnets to:

| Subnet id | Public | Description |
| --------- | ------ | ----------- |
{}",
        subnets
            .values()
            .map(|s| {
                let excluded_desc = excluded_subnets.get(&s.principal);
                format!(
                    "| {} | {} | {} |",
                    s.principal,
                    excluded_desc.is_none(),
                    excluded_desc.map(|s| s.to_string()).unwrap_or_default()
                )
            })
            .join("\n")
    ))
}
