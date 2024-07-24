use std::{path::PathBuf, process::Stdio, str::FromStr};

use itertools::Itertools;
use log::info;
use serde_json::Value;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::mpsc::Sender,
};
use tokio_util::sync::CancellationToken;

use crate::Message;

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
    loop {
        let line = stdout_reader.next_line().await?;
        if let Some(line) = line {
            whole_config.push(line.trim().to_string());
            let config = whole_config.iter().join("");

            if serde_json::from_str::<Value>(&config).is_ok() {
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
    token.cancelled().await;
    info!("Received shutdown, killing testnet");
    child.kill().await?;

    Ok(())
}
