use std::path::PathBuf;

use slog::{debug, info, Logger};
use tokio::{
    fs::{create_dir_all, File},
    io::AsyncWriteExt,
    process::Command,
};

pub async fn sync_git(logger: &Logger, path: &PathBuf, url: &String, release_index: &String) -> anyhow::Result<()> {
    if !path.exists() {
        info!(logger, "Git directory not found. Creating...");
        create_dir_all(path)
            .await
            .map_err(|e| anyhow::anyhow!("Couldn't create directory for git repo: {:?}", e))?;
        debug!(logger, "Created directory for github repo at: '{}'", path.display());

        configure_git_repo(path, url, &logger, release_index).await?;
        debug!(logger, "Repo configured")
    }

    debug!(logger, "Syncing git repo");
    execute_git_command(&path, &["pull"]).await
}

async fn configure_git_repo(
    path: &PathBuf,
    url: &String,
    logger: &Logger,
    release_index: &String,
) -> anyhow::Result<()> {
    debug!(logger, "Initializing repository on path '{}'", path.display());
    execute_git_command(path, &["init"]).await?;
    debug!(logger, "Configuring sparse checkout");
    execute_git_command(path, &["config", "core.sparseCheckout", "true"]).await?;
    debug!(logger, "Setting up remote");
    execute_git_command(path, &["remote", "add", "-f", "origin", url]).await?;

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
