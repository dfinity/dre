use std::str::FromStr;

use anyhow::Error;
use itertools::Itertools;
use log::info;
use serde_json::Value;
use tokio::sync::mpsc::Receiver;

use crate::Message;

pub async fn qualify(receiver: &mut Receiver<Message>) -> anyhow::Result<()> {
    // Run dre to qualify with correct parameters
    info!("Awaiting logs path...");
    let data = receiver.recv().await.ok_or(anyhow::anyhow!("Failed to recv data"))?;

    info!("Received logs: {}", data);

    info!("Awaiting config...");
    let data = receiver.recv().await.ok_or(anyhow::anyhow!("Failed to recv data"))?;

    let config = match data {
        Message::Log(_) => anyhow::bail!("Expected `Config` but found `Log`"),
        Message::Config(c) => c,
    };
    let config = Config::from_str(&config)?;

    info!("Received following config: {:?}", config);
    info!("Running qualification...");

    Ok(())
}

#[derive(Debug)]
struct Config {
    deployment_name: String,
    kibana_url: String,
    nns_urls: Vec<String>,
    prometheus_url: String,
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
            prometheus_url: format!("http://prometheus.{}.testnet.farm.dfinity.systems", deployment_name),
            deployment_name,
            kibana_url: parsed["kibana_url"]["url"]
                .as_str()
                .ok_or(anyhow::anyhow!("Failed to find 'kibana_url.url'"))?
                .to_string(),
            nns_urls: nns_urls.into_iter().map(|n| format!("http://[{}]:8080/", n.unwrap())).collect_vec(),
        };

        Ok(config)
    }
}
