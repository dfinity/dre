use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    str::FromStr,
    time::Duration,
};

use clap::{error::ErrorKind, Args};
use ic_management_types::Network;
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use log::info;
use reqwest::Client;
use serde_json::Value;

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

        let state_metrics = self.fetch_subnet_state_metrics(ctx.network()).await?;

        let registry = ctx.registry().await;
        let subnets = registry.subnets().await?;
        let mut excluded_subnets = BTreeMap::new();

        let human_bytes = human_bytes::human_bytes(DEFAULT_STATE_SIZE_BYTES_LIMIT as f64);

        for subnet in subnets.values() {
            if subnet.subnet_type.eq(&SubnetType::System) {
                excluded_subnets.insert(subnet.principal.clone(), "System subnet".to_string());
                continue;
            }

            let subnet_principal_string = subnet.principal.to_string();
            if let Some((_, description)) = csv_contents.iter().find(|(short_id, _)| subnet_principal_string.starts_with(short_id)) {
                excluded_subnets.insert(subnet.principal.clone(), description.to_owned());
                continue;
            }

            let subnet_state = state_metrics.get(&subnet.principal).ok_or(anyhow::anyhow!(
                "Didn't find state metric for subnet id: {}",
                subnet.principal.to_string()
            ))?;

            if subnet_state >= &self.state_size_limit {
                excluded_subnets.push(ExcludedSubnet {
                    subnet_id: subnet.principal.clone(),
                    description: format!("Subnet has over {} of state", human_bytes),
                });
                continue;
            }

            if self.subnet_canister_num_over_limit(&subnet.principal).await? {
                excluded_subnets.push(ExcludedSubnet {
                    subnet_id: subnet.principal.clone(),
                    description: format!("Subnet has over {} canisters", self.canister_limit),
                });
                continue;
            }
        }

        Ok(())
    }
}

const DASHBOARD_CANISTER_API: &str = "https://ic-api.internetcomputer.org/api/v3/canisters";
const STATE_METRIC: &str = "state_manager_state_size_bytes";

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

    async fn subnet_canister_num_over_limit(&self, subnet_id: &PrincipalId) -> anyhow::Result<bool> {
        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

        let response = client
            .get(DASHBOARD_CANISTER_API)
            .query(&[("limit", "0"), ("subnet_id", subnet_id.to_string().as_str())])
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        let canister_num = response["total_canisters"].as_u64().ok_or(anyhow::anyhow!(
            "Missing `total_canisters` field in response body:\n{}",
            serde_json::to_string_pretty(&response)?
        ))?;

        Ok(self.canister_limit <= canister_num)
    }

    async fn fetch_subnet_state_metrics(&self, network: &Network) -> anyhow::Result<BTreeMap<PrincipalId, u64>> {
        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

        let response = client
            .get(network.get_prometheus_endpoint().join("api/v1/query")?)
            .query(&[("query", format!("sum({}) by (ic_subnet)", STATE_METRIC))])
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;
        let response = response["data"]["result"].as_array().ok_or(anyhow::anyhow!(
            "Didn't find `data.result` within response: \n{}",
            serde_json::to_string_pretty(&response)?
        ))?;
        let mut ret = BTreeMap::new();

        for entry in response {
            let subnet_id = entry["metric"]["ic_subnet"].as_str().ok_or(anyhow::anyhow!(
                "Failed to find subnet id within structure:\n{}",
                serde_json::to_string_pretty(&entry)?
            ))?;

            let state_size_bytes = entry["value"]
                .as_array()
                .ok_or(anyhow::anyhow!(
                    "Failed to parse metric value for result:\n{}",
                    serde_json::to_string_pretty(&entry)?
                ))?
                .get(1)
                .ok_or(anyhow::anyhow!(
                    "Array doesn't contain expected number of elements:\n{}",
                    serde_json::to_string_pretty(&entry)?
                ))?
                .as_u64()
                .ok_or(anyhow::anyhow!("Value isn't a u64:\n{}", serde_json::to_string_pretty(&entry)?))?;

            ret.insert(PrincipalId::from_str(subnet_id)?, state_size_bytes);
        }

        Ok(ret)
    }
}

struct ExcludedSubnet {
    subnet_id: PrincipalId,
    description: String,
}
