use std::{path::PathBuf, time::Duration};

use slog::{debug, info, Logger};
use tokio::{
    fs::{create_dir_all, File},
    io::AsyncWriteExt,
    process::Command,
    select,
    sync::broadcast::Receiver,
};

pub async fn watch_git(
    logger: Logger,
    path: PathBuf,
    poll_interval: Duration,
    mut shutdown: Receiver<()>,
    url: String,
    release_index: String,
) -> anyhow::Result<()> {
    if !path.exists() {
        info!(logger, "Git directory not found. Creating...");
        select! {
            _ = shutdown.recv() => {
                info!(logger, "Received shutdown in 'git' thread");
                return Ok(());
            }
            res = create_dir_all(&path) => match res {
                Ok(_) => debug!(logger, "Created directory for github repo at: '{}'", path.display()),
                Err(e) => return Err(anyhow::anyhow!("Couldn't create directory for git repo: {:?}", e))
            }
        }
    }

    // Configure directory
    select! {
        _ = shutdown.recv() => {
            info!(logger, "Received shutdown in 'git' thread");
            return Ok(());
        }
        res = configure_git_repo(&path, url, logger.clone(), release_index) => match res {
            Ok(_) => debug!(logger, "Repo configured"),
            Err(e) => return Err(e),
        }
    }

    // Loop and pool
    let mut interval = tokio::time::interval(poll_interval);
    loop {
        select! {
            tick = interval.tick() => {
                debug!(logger, "Received tick: {:?}", tick)
            }
            _ = shutdown.recv() => {
                info!(logger, "Received shutdown in 'git' thread");
                return Ok(());
            }
        }

        debug!(logger, "Syncing git repo");
        execute_git_command(&path, &["pull"]).await?
    }
}

async fn configure_git_repo(path: &PathBuf, url: String, logger: Logger, release_index: String) -> anyhow::Result<()> {
    debug!(logger, "Initializing repository on path '{}'", path.display());
    execute_git_command(path, &["init"]).await?;
    debug!(logger, "Configuring sparse checkout");
    execute_git_command(path, &["config", "core.sparseCheckout", "true"]).await?;
    debug!(logger, "Setting up remote");
    execute_git_command(path, &["remote", "add", "-f", "origin", &url]).await?;

    debug!(logger, "Creating file for sparse checkout paths");
    let mut file = File::create(path.join(".git/info/sparse-checkout"))
        .await
        .map_err(|e| anyhow::anyhow!("Couldn't create sparse-checkout file: {:?}", e))?;

    debug!(logger, "Writing release index path to sparse checkout");
    file.write_all(release_index.as_bytes())
        .await
        .map_err(|e| anyhow::anyhow!("Couldn't write to sparse-checkout file: {:?}", e))?;

    debug!(logger, "Checking out 'main'");
    execute_git_command(path, &["checkout", "main"]).await?;

    Ok(())
}

async fn execute_git_command(path: &PathBuf, args: &[&str]) -> anyhow::Result<()> {
    let mut cmd = Command::new("git");
    cmd.current_dir(path);
    cmd.args(args.iter());
    cmd.output()
        .await
        .map_err(|e| anyhow::anyhow!("Couldn't execute command 'git {}': {:?}", args.join(" "), e))?;
    Ok(())
}
