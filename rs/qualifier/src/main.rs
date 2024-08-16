use std::{fmt::Display, path::PathBuf, str::FromStr, time::Duration};

use clap::Parser;
use cli::Args;
use ict_util::ict;
use log::info;
use qualify_util::qualify;
use reqwest::ClientBuilder;
use serde_json::Value;
use std::io::Write;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use version_selector::StartVersionSelectorBuilder;

mod cli;
mod ict_util;
mod qualify_util;
mod version_selector;

const NETWORK_NAME: &str = "configured-testnet";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger();

    let args = Args::parse();
    if args.version_to_qualify.is_empty() {
        anyhow::bail!("Version to qualify is required, but empty string is passed.");
    }
    info!("Running qualification for {}", args.version_to_qualify);
    info!("Generating keys for farm testnets...");
    let (neuron_id, private_key_pem) = args.ensure_test_key()?;
    info!("Principal key created");

    args.ensure_xnet_test_key()?;
    // Take in one version and figure out what is the base version
    //
    // To find the initial version we could take NNS version?
    let initial_version = if let Some(ref v) = args.initial_version {
        v.to_string()
    } else {
        info!("Fetching the forcasted version of NNS which will be used as starting point");
        // Fetch the starter versions
        let start_version_selector = StartVersionSelectorBuilder::new()
            .with_client(ClientBuilder::new().connect_timeout(Duration::from_secs(30)))
            .build()
            .await?;

        start_version_selector.get_forcasted_version_for_mainnet_nns()?
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

    let artifacts = PathBuf::from_str("/tmp/qualifier-artifacts")?.join(&args.version_to_qualify);
    info!("Will store artifacts in: {}", artifacts.display());
    std::fs::create_dir_all(&artifacts)?;
    if artifacts.exists() {
        info!("Making sure artifact store is empty");
        std::fs::remove_dir_all(&artifacts)?;
        std::fs::create_dir(&artifacts)?;
    }

    let mut file = std::fs::File::create_new(artifacts.join("ic-config.json"))?;
    writeln!(file, "{}", &config)?;

    tokio::select! {
        res = ict(args.ic_repo_path.clone(), config, token.clone(), sender) => res?,
        res = qualify(
            &mut receiver,
            private_key_pem,
            neuron_id,
            NETWORK_NAME,
            initial_version,
            args.version_to_qualify.to_string(),
            artifacts,
            args.step_range
        ) => res?
    };

    info!("Finished qualifier run for: {}", args.version_to_qualify);

    token.cancel();
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
