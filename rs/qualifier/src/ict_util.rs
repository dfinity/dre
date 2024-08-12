use std::{path::PathBuf, process::Stdio, str::FromStr, time::Duration};

use itertools::Itertools;
use log::info;
use reqwest::ClientBuilder;
use serde_json::Value;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::mpsc::Sender,
};
use tokio_util::sync::CancellationToken;

use crate::Message;

// Each 30s set the ttl for a testnet to 90 seconds
const FARM_GROUP_KEEPALIVE_TTL_SEC: u64 = 90;
const KEEPALIVE_PERIOD: Duration = Duration::from_secs(30);
const KEEPALIVE_PERIOD_ERROR: Duration = Duration::from_secs(5);
pub const FARM_BASE_URL: &str = "https://farm.dfinity.systems";

pub async fn ict(ic_git: PathBuf, config: String, token: CancellationToken, sender: Sender<Message>) -> anyhow::Result<()> {
    let ic_config = PathBuf::from_str("/tmp/ic_config.json")?;
    std::fs::write(&ic_config, &config)?;

    let command = "gitlab-ci/container/container-run.sh";
    let args = &[
        "ict",
        "testnet",
        "create",
        "--lifetime-mins",
        "180",
        "--from-ic-config-path",
        &ic_config.display().to_string(),
    ];

    info!("From directory: {}", ic_git.display());
    info!("Running command: {} {}", command, args.iter().join(" "));
    let mut child = Command::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(&ic_git)
        .spawn()?;

    let mut stdout = child.stdout.take().ok_or(anyhow::anyhow!("Couldn't take stdout"))?;
    let mut stdout_reader = BufReader::new(&mut stdout).lines();

    let target = "Testnet is being deployed, please wait ...";
    let logs;
    info!("Finding logs file...");
    loop {
        let line = stdout_reader.next_line().await?;
        if let Some(text) = line {
            if text.contains(target) {
                let path = text
                    .split(target)
                    .last()
                    .ok_or(anyhow::anyhow!("Failed to parse output"))?
                    .trim()
                    .to_string();
                logs = path;
                break;
            }
        }

        if token.is_cancelled() {
            return Ok(());
        }

        match child.try_wait() {
            Ok(Some(s)) => {
                let stderr = child.stderr.take().ok_or(anyhow::anyhow!("Unable to get stderr"))?;
                let mut lines = BufReader::new(stderr).lines();

                let mut all = vec![];
                while let Some(line) = lines.next_line().await? {
                    all.push(line)
                }

                anyhow::bail!("Finished early with status code {:?} and error: \n{}", s, all.iter().join("\n"))
            }
            _ => continue,
        }
    }

    sender
        .send(Message::Log(logs))
        .await
        .map_err(|_| anyhow::anyhow!("Failed to send data across channels"))?;

    info!("Building testnet...");
    let mut whole_config = vec![];
    let deployment_name;
    loop {
        let line = stdout_reader.next_line().await?;
        if let Some(line) = line {
            whole_config.push(line.trim().to_string());
            let config = whole_config.iter().join("");

            if let Ok(v) = serde_json::from_str::<Value>(&config) {
                deployment_name = v["farm"]["group"]
                    .as_str()
                    .ok_or(anyhow::anyhow!("Failed to find 'farm.group'"))?
                    .to_string();
                break;
            }
        }

        if token.is_cancelled() {
            return Ok(());
        }

        match child.try_wait() {
            Ok(Some(s)) => {
                let stderr = child.stderr.take().ok_or(anyhow::anyhow!("Unable to get stderr"))?;
                let mut lines = BufReader::new(stderr).lines();

                let mut all = vec![];
                while let Some(line) = lines.next_line().await? {
                    all.push(line)
                }

                anyhow::bail!("Finished early with status code {:?} and error: \n{}", s, all.iter().join("\n"))
            }
            _ => continue,
        }
    }

    let config = whole_config.iter().join("");
    sender
        .send(Message::Config(config))
        .await
        .map_err(|_| anyhow::anyhow!("Failed to send data across channels"))?;

    child.stdout = Some(stdout);

    if std::env::var("MANUALY_TTL_FARM").is_ok() {
        tokio::select! {
            _ = keep_testnet_alive(deployment_name, token.clone()) => {},
            _ = token.cancelled() => {}
        }
    } else {
        token.cancelled().await;
    }

    info!("Received shutdown, killing testnet");
    child.kill().await?;

    Ok(())
}

async fn keep_testnet_alive(group_name: String, token: CancellationToken) -> anyhow::Result<()> {
    let client = ClientBuilder::new().timeout(Duration::from_secs(15)).build()?;
    let ttl_url = format!("{}/group/{}/ttl/{}", FARM_BASE_URL, &group_name, FARM_GROUP_KEEPALIVE_TTL_SEC);
    info!("Will be using following ttl link: {}", ttl_url);

    while !token.is_cancelled() {
        let resp_future = client.put(&ttl_url).send().await;

        match resp_future {
            Ok(r) => match r.error_for_status() {
                Ok(_) => tokio::time::sleep(KEEPALIVE_PERIOD).await,
                _ => tokio::time::sleep(KEEPALIVE_PERIOD_ERROR).await,
            },
            _ => tokio::time::sleep(KEEPALIVE_PERIOD_ERROR).await,
        }
    }

    Ok(())
}
