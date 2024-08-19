use std::{
    fmt::Display,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

use clap::Parser;
use cli::Args;
use futures::future::join_all;
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

    let initial_versions = if let Some(ref v) = args.initial_versions {
        v
    } else {
        info!("Fetching the forecasted versions from mainnet which will be used as starting point");
        // Fetch the starter versions
        let start_version_selector = StartVersionSelectorBuilder::new()
            .with_client(ClientBuilder::new().connect_timeout(Duration::from_secs(30)))
            .build()
            .await?;

        &start_version_selector.get_forecasted_versions_from_mainnet()?
    };

    info!("Initial versions that will be used: {}", initial_versions.join(","));

    args.ensure_git().await?;

    let artifacts = PathBuf::from_str("/tmp/qualifier-artifacts")?.join(&args.version_to_qualify);
    info!("Will store artifacts in: {}", artifacts.display());
    std::fs::create_dir_all(&artifacts)?;
    if artifacts.exists() {
        info!("Making sure artifact store is empty");
        std::fs::remove_dir_all(&artifacts)?;
        std::fs::create_dir(&artifacts)?;
    }

    info!("Qualification will run in {} mode", args.mode);
    let outcomes = match args.mode {
        cli::QualificationMode::Sequential => {
            let mut outcomes = vec![];
            for iv in initial_versions {
                let current_path = &artifacts.join(format!("from-{}", iv));
                if let Err(e) = std::fs::create_dir(current_path) {
                    outcomes.push(Err(anyhow::anyhow!(e)))
                }
                outcomes.push(run_qualification(&args, iv.clone(), current_path, neuron_id, &private_key_pem).await)
            }
            outcomes
        }
        cli::QualificationMode::Parallel => {
            join_all(initial_versions.iter().map(|iv| async {
                let current_path = &artifacts.join(format!("from-{}", iv.clone()));
                if let Err(e) = std::fs::create_dir(current_path) {
                    return Err(anyhow::anyhow!(e));
                };
                run_qualification(&args, iv.clone(), current_path, neuron_id, &private_key_pem).await
            }))
            .await
        }
    };

    if outcomes.iter().any(|o| o.is_err()) {
        anyhow::bail!("Overall qualification failed due to one or more sub-qualifications failing")
    }

    Ok(())
}

async fn run_qualification(args: &Args, initial_version: String, artifacts: &Path, neuron_id: u64, private_key_pem: &Path) -> anyhow::Result<()> {
    if initial_version == args.version_to_qualify {
        anyhow::bail!("Starting version and version being qualified are the same: {}", args.version_to_qualify)
    }

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
            initial_version
        );

        // Validate that the string is valid json
        serde_json::to_string_pretty(&serde_json::from_str::<Value>(&config)?)?
    };
    info!("[{} -> {}]: Using configuration: \n{}", initial_version, args.version_to_qualify, config);

    // Run ict and capture its output
    //
    // Its important to parse the output correctly so we get the path to
    // log of the tool if something fails, on top of that we should
    // aggregate the output of the command which contains the json dump
    // of topology to parse it and get the nns urls and other links. Also
    // we have to extract the neuron pem file to use with dre
    let token = CancellationToken::new();
    let (sender, mut receiver) = mpsc::channel(2);

    let mut file = std::fs::File::create_new(artifacts.join("ic-config.json"))?;
    writeln!(file, "{}", &config)?;
    let current_network_name = format!("{}-{}", NETWORK_NAME, initial_version);

    tokio::select! {
        res = ict(args.ic_repo_path.clone(), token.clone(), sender, artifacts.to_path_buf()) => res?,
        res = qualify(
            &mut receiver,
            private_key_pem.to_path_buf(),
            neuron_id,
            current_network_name.as_str(),
            initial_version.to_owned(),
            args.version_to_qualify.to_string(),
            artifacts.to_path_buf(),
            args.step_range.clone()
        ) => res?
    };

    info!("Finished qualifier run for: {} -> {}", initial_version, args.version_to_qualify);

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
