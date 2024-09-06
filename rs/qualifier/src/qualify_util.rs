use std::{path::PathBuf, str::FromStr};

use anyhow::Error;
use dre::{
    commands::{qualify::execute::Execute, ExecutableCommand},
    ctx::DreContext,
};
use ic_management_backend::registry::local_registry_path;
use ic_management_types::Network;
use itertools::Itertools;
use log::info;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::mpsc::Receiver;
use url::Url;

use crate::Message;

pub async fn qualify(
    receiver: &mut Receiver<Message>,
    private_key_pem: PathBuf,
    neuron_id: u64,
    network_name: &str,
    from_version: String,
    to_version: String,
    artifacts: PathBuf,
    step_range: Option<String>,
) -> anyhow::Result<()> {
    // Run dre to qualify with correct parameters
    info!("Awaiting logs path...");
    let data = receiver.recv().await.ok_or(anyhow::anyhow!("Failed to recv data"))?;
    let log_path = match data {
        Message::Log(line) => {
            let log_path = line
                .split_once('/')
                .map(|(_, last)| format!("/{}", last[..last.len() - 1].to_owned()))
                .ok_or(anyhow::anyhow!("Expected log line"))?;
            info!("Log file path: {}", log_path);
            PathBuf::from_str(&log_path)?
        }
        _ => anyhow::bail!("Expected Log line instead of data"),
    };

    info!("Awaiting config...");
    let data = receiver.recv().await.ok_or(anyhow::anyhow!("Failed to recv data"))?;

    let config = match data {
        Message::Log(_) => anyhow::bail!("Expected `Config` but found `Log`"),
        Message::Config(c) => c,
    };
    let config = Config::from_str(&config)?;

    info!("Received following config: {}", serde_json::to_string_pretty(&config)?);
    info!("Running qualification...");

    // At this point we are going to run so we need to remove previous
    // registry stored on the disk
    let reg_path = local_registry_path(&Network::new_unchecked(network_name, &config.nns_urls).unwrap());
    if reg_path.exists() {
        info!("Detected registry from previous runs on path: {}", reg_path.display());
        std::fs::remove_dir_all(&reg_path)?;
        info!("Removed registry from previous runs");
    }

    let cmd = dre::commands::Subcommands::Qualify(dre::commands::qualify::Qualify {
        subcommand: dre::commands::qualify::Subcommands::Execute(Execute {
            version: to_version,
            from_version: Some(from_version),
            step_range,
            deployment_name: config.deployment_name,
            prometheus_endpoint: config.prometheus_url,
            artifacts: Some(artifacts.clone()),
            grafana_url: Some(config.grafana_url),
        }),
    });
    let ctx = DreContext::new(
        network_name.to_string(),
        config.nns_urls,
        private_key_pem.try_into()?,
        Some(neuron_id),
        false,
        false,
        true,
        false,
        cmd.require_auth(),
        None,
        dre::commands::IcAdminVersion::FromGovernance,
    )
    .await?;

    cmd.execute(ctx).await?;

    std::fs::copy(&log_path, artifacts.join("farm-driver.log"))?;
    Ok(())
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct Config {
    deployment_name: String,
    kibana_url: String,
    nns_urls: Vec<Url>,
    prometheus_url: String,
    grafana_url: String,
}

impl FromStr for Config {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parsed = serde_json::from_str::<Value>(s)?;

        let system = parsed["ic_topology"]["subnets"]
            .as_array()
            .ok_or(anyhow::anyhow!("Failed to find 'ic_topology.subnets'"))?
            .iter()
            .find(|elem| elem["subnet_type"].as_str().eq(&Some("system")))
            .ok_or(anyhow::anyhow!("Didn't find system subnet"))?;

        let nns_urls = system["nodes"]
            .as_array()
            .ok_or(anyhow::anyhow!("Didn't find nodes within system subnet"))?
            .iter()
            .map(|n| n["ipv6"].as_str().ok_or(anyhow::anyhow!("Didn't find ipv6 within node")))
            .collect_vec();

        if nns_urls.iter().any(|res| res.is_err()) {
            anyhow::bail!("Failed to deserialize nns urls")
        }

        let deployment_name = parsed["farm"]["group"]
            .as_str()
            .ok_or(anyhow::anyhow!("Failed to find 'farm.group'"))?
            .to_string();

        let config = Self {
            prometheus_url: format!("http://prometheus.{}.testnet.farm.dfinity.systems/api/v1/query", deployment_name),
            grafana_url: format!("http://grafana.{}.testnet.farm.dfinity.systems/", deployment_name),
            deployment_name,
            kibana_url: parsed["kibana_url"]["url"]
                .as_str()
                .ok_or(anyhow::anyhow!("Failed to find 'kibana_url.url'"))?
                .to_string(),
            nns_urls: nns_urls
                .into_iter()
                .map(|n| Url::from_str(&format!("http://[{}]:8080/", n.unwrap())).unwrap())
                .collect_vec(),
        };

        Ok(config)
    }
}
