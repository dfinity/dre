use std::path::PathBuf;

use slog::{debug, info, Logger};
use tokio::{
    fs::{create_dir_all, File},
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};

use crate::rollout_schedule::Index;

use super::RolloutScheduleFetcher;

#[derive(Clone)]
pub struct SparseCheckoutFetcher {
    logger: Logger,
    path: PathBuf,
    release_index: String,
}

impl SparseCheckoutFetcher {
    pub async fn new(logger: Logger, path: PathBuf, url: String, release_index: String) -> anyhow::Result<Self> {
        let fetcher = Self {
            logger,
            path,
            release_index,
        };

        if !fetcher.path.exists() {
            info!(fetcher.logger, "Git directory not found. Creating...");
            create_dir_all(&fetcher.path)
                .await
                .map_err(|e| anyhow::anyhow!("Couldn't create directory for git repo: {:?}", e))?;
            debug!(
                fetcher.logger,
                "Created directory for github repo at: '{}'",
                fetcher.path.display()
            );

            fetcher.configure_git_repo(&url).await?;
            debug!(fetcher.logger, "Repo configured")
        }

        Ok(fetcher)
    }

    async fn configure_git_repo(&self, url: &String) -> anyhow::Result<()> {
        debug!(self.logger, "Initializing repository on path '{}'", self.path.display());
        Self::execute_git_command(&self.path, &["init"]).await?;
        debug!(self.logger, "Configuring sparse checkout");
        Self::execute_git_command(&self.path, &["config", "core.sparseCheckout", "true"]).await?;
        debug!(self.logger, "Setting up remote");
        Self::execute_git_command(&self.path, &["remote", "add", "-f", "origin", url]).await?;

        debug!(self.logger, "Creating file for sparse checkout paths");
        let mut file = File::create(self.path.join(".git/info/sparse-checkout"))
            .await
            .map_err(|e| anyhow::anyhow!("Couldn't create sparse-checkout file: {:?}", e))?;

        debug!(self.logger, "Writing release index path to sparse checkout");
        file.write_all(self.release_index.as_bytes())
            .await
            .map_err(|e| anyhow::anyhow!("Couldn't write to sparse-checkout file: {:?}", e))?;

        debug!(self.logger, "Checking out 'main'");
        Self::execute_git_command(&self.path, &["checkout", "main"]).await?;

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
}

impl RolloutScheduleFetcher for SparseCheckoutFetcher {
    async fn fetch(&self) -> anyhow::Result<Index> {
        debug!(self.logger, "Syncing git repo");
        SparseCheckoutFetcher::execute_git_command(&self.path, &["pull"]).await?;

        debug!(self.logger, "Deserializing index");
        let mut index = String::from("");
        let mut rel_index = File::open(self.path.join(&self.release_index))
            .await
            .map_err(|e| anyhow::anyhow!("Couldn't open release index: {:?}", e))?;
        rel_index
            .read_to_string(&mut index)
            .await
            .map_err(|e| anyhow::anyhow!("Couldn't read release index: {:?}", e))?;
        serde_yaml::from_str(&index).map_err(|e| anyhow::anyhow!("Couldn't parse release index: {:?}", e))
    }
}
