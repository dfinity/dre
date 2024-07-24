use std::{fmt::Display, time::Duration};

use clap::Parser;
use cli::Args;
use dre::{
    auth::{Auth, Neuron},
    ic_admin::{download_ic_admin, should_update_ic_admin, IcAdminWrapper},
};
use ic_canisters::governance::governance_canister_version;
use ic_management_types::Network;
use ict_util::ict;
use log::info;
use qualify_util::qualify;
use reqwest::ClientBuilder;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

mod cli;
mod ict_util;
mod qualify_util;

const NETWORK_NAME: &str = "configured-testnet";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger();

    // Check if farm is reachable. If not, error
    let client = ClientBuilder::new().timeout(Duration::from_secs(30)).build()?;
    client
        .get("https://kibana.testnet.dfinity.network")
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Checking connectivity failed: {}", e.to_string()))?
        .error_for_status()
        .map_err(|e| anyhow::anyhow!("Checking connectivity failed: {}", e.to_string()))?;

    let args = Args::parse();
    info!("Running qualification for {}", args.version_to_qualify);
    info!("Generating keys for farm testnets...");
    let (neuron_id, private_key_pem) = args.ensure_key()?;
    info!("Principal key created");

    // Take in one version and figure out what is the base version
    //
    // To find the initial version we could take NNS version?
    let initial_version = if let Some(ref v) = args.initial_version {
        v.to_string()
    } else {
        info!("Fetching the version of NNS which will be used as starting point");
        let mainnet = Network::new_unchecked("mainnet", &[])?;
        let ic_admin_path = match should_update_ic_admin()? {
            (true, _) => {
                let govn_canister_version = governance_canister_version(mainnet.get_nns_urls()).await?;
                download_ic_admin(Some(govn_canister_version.stringified_hash)).await?
            }
            (false, s) => s,
        };
        let mainnet_anonymous_neuron = Neuron {
            auth: Auth::Anonymous,
            neuron_id: 0,
            include_proposer: false,
        };
        let ic_admin = IcAdminWrapper::new(mainnet, Some(ic_admin_path), true, mainnet_anonymous_neuron, false);
        let response = ic_admin
            .run_passthrough_get(
                &[
                    "subnet".to_string(),
                    "tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe".to_string(),
                ],
                true,
            )
            .await?;

        let initial_version = serde_json::from_str::<Value>(&response)?;
        let initial_version = initial_version["records"][0]["value"]["replica_version_id"]
            .as_str()
            .ok_or(anyhow::anyhow!("Couldn't parse subnet record"))?;
        initial_version.to_string()
    };

    if initial_version == args.version_to_qualify {
        anyhow::bail!("Initial version and version to qualify are the same")
    }
    info!("Initial version that will be used: {}", initial_version);

    // Generate configuration for `ict` including the initial version
    //
    // We could take in a file and mutate it and copy it to /tmp folder
    let config = if let Some(ref path) = args.config_override {
        let contents = std::fs::read_to_string(path)?;
        let mut config = serde_json::from_str::<Value>(&contents)?;
        config["initial_version"] = serde_json::Value::String(initial_version.to_owned());

        serde_json::to_string_pretty(&config)?
    } else {
        let config = format!(
            r#"{{
            "subnets": [
            {{
              "subnet_type": "application",
              "num_nodes": 4
            }},
            {{
              "subnet_type": "application",
              "num_nodes": 4
            }},
            {{
              "subnet_type": "system",
              "num_nodes": 4
            }}
          ],
          "num_unassigned_nodes": 4,
          "initial_version": "{}"
        }}"#,
            &initial_version
        );

        // Validate that the string is valid json
        serde_json::to_string_pretty(&serde_json::from_str::<Value>(&config)?)?
    };
    info!("Using configuration: \n{}", config);

    args.ensure_git().await?;

    // Run ict and capture its output
    //
    // Its important to parse the output correctly so we get the path to
    // log of the tool if something fails, on top of that we should
    // aggregate the output of the command which contains the json dump
    // of topology to parse it and get the nns urls and other links. Also
    // we have to extract the neuron pem file to use with dre
    let token = CancellationToken::new();
    let (sender, mut receiver) = mpsc::channel(2);
    let handle = tokio::spawn(ict(args.ic_repo_path.clone(), config, token.clone(), sender));

    qualify(
        &mut receiver,
        private_key_pem,
        neuron_id,
        NETWORK_NAME,
        initial_version,
        args.version_to_qualify.to_string(),
    )
    .await?;

    info!("Finished qualifier run for: {}", args.version_to_qualify);

    token.cancel();
    handle.await??;
    Ok(())
}

fn init_logger() {
    match std::env::var("RUST_LOG") {
        Ok(val) => std::env::set_var("LOG_LEVEL", val),
        Err(_) => {
            if std::env::var("LOG_LEVEL").is_err() {
                // Default logging level is: info generally, warn for mio and actix_server
                // You can override defaults by setting environment variables
                // RUST_LOG or LOG_LEVEL
                std::env::set_var("LOG_LEVEL", "info,mio::=warn,actix_server::=warn")
            }
        }
    }
    pretty_env_logger::init_custom_env("LOG_LEVEL");
}

pub enum Message {
    Log(String),
    Config(String),
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match &self {
                Message::Log(p) => p,
                Message::Config(c) => c,
            }
        )
    }
}
