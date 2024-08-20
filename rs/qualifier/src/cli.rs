use std::{path::PathBuf, process::Stdio, str::FromStr};

use clap::Parser;

use ic_nervous_system_common_test_keys::TEST_NEURON_1_OWNER_KEYPAIR;
use strum::Display;
use tokio::process::Command;
const TEST_NEURON_1_IDENTITY_PATH: &str = ".config/dfx/identity/test_neuron_1/identity.pem";
const XNET_TESTING_IDENTITY_PATH: &str = ".config/dfx/identity/xnet-testing/identity.pem";

#[derive(Parser)]
#[clap(about, version)]
pub struct Args {
    /// Version to qualify
    pub version_to_qualify: String,

    /// Specify a list of versions from which the qualification
    /// should start. The default will be the same forecasted
    /// versions that will endup on mainnet after the active
    /// rollout is finished.
    ///
    /// The information is gathered from https://rollout-dashboard.ch1-rel1.dfinity.network/api/v1/rollouts
    #[clap(long)]
    pub initial_versions: Option<Vec<String>>,

    /// Path which contains the layout of the network to
    /// be deployed. The default value will be a network
    /// consisting of:
    ///   2 application subnets (4 nodes per subnet)
    ///   1 system subnet (4 nodes)
    ///   4 unassigned nodes
    #[clap(long)]
    pub config_override: Option<PathBuf>,

    #[clap(long, default_value = dirs::cache_dir().unwrap().join("git/ic").display().to_string())]
    pub ic_repo_path: PathBuf,

    /// Skip the pulling of ic repo which is mostly useful
    /// for development since each change on master will
    /// result in rebuilding of image
    #[clap(long)]
    pub skip_pull: bool,

    /// Specify the steps to run
    /// A range can be: `4`, `3..`, `..3, `1..3`
    #[clap(long)]
    pub step_range: Option<String>,

    /// If there are multiple forecasted versions on the network at
    /// the end of an active rollout this controls how the qualification
    /// will run.
    #[clap(long, default_value_t = QualificationMode::Sequential)]
    pub mode: QualificationMode,
}

#[derive(Display, Clone, clap::ValueEnum)]
#[strum(serialize_all = "snake_case")]
pub enum QualificationMode {
    /// Less invasive towards farm, but slower.
    ///
    /// If default config is used this means 16 vm's
    /// Each qualification is run in sequence and
    /// observed time for one qualification is roughly
    /// 1h 30mins, meaning that if there is more than
    /// 2 beginning versions qualification can take up
    /// to 5 hours to complete.
    Sequential,
    /// More invasive towards farm, but faster.
    ///
    /// If the default config is used this means that
    /// qualifier will spin up N amount of networks
    /// where N is the number of start versions for
    /// qualification. Each network (for the default config)
    /// will take 16 vm's meaning that in total qualifier
    /// will take 16 * N vm's.
    Parallel,
}

impl Args {
    pub fn ensure_test_key(&self) -> anyhow::Result<(u64, PathBuf)> {
        let key_pair = &TEST_NEURON_1_OWNER_KEYPAIR;
        let path = dirs::home_dir()
            .ok_or(anyhow::anyhow!("No home dir present"))?
            .join(PathBuf::from_str(TEST_NEURON_1_IDENTITY_PATH)?);
        let dir = path.parent().ok_or(anyhow::anyhow!("No parent dir for path: {}", path.display()))?;
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
        }

        std::fs::write(&path, key_pair.to_pem()).map_err(|e| anyhow::anyhow!(e))?;
        // TODO: When we upgrade ic repo there will be a constant for this
        Ok((449479075714955186, path))
    }

    pub fn ensure_xnet_test_key(&self) -> anyhow::Result<()> {
        let path = dirs::home_dir()
            .ok_or(anyhow::anyhow!("No home dir present"))?
            .join(PathBuf::from_str(XNET_TESTING_IDENTITY_PATH)?);
        match path.exists() {
            true => {
                let metadata = path.metadata()?;
                match metadata.len() {
                    0 => anyhow::bail!("Xnet-testing identity is present on path {} but is empty", path.display()),
                    _ => Ok(()),
                }
            }
            false => anyhow::bail!("Missing xnet-testing identity on path: {}", path.display()),
        }
    }

    pub async fn ensure_git(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.ic_repo_path)?;

        let git_dir = &self.ic_repo_path.join(".git");
        if !git_dir.exists() {
            if !Command::new("git")
                .args(["clone", "https://github.com/dfinity/ic.git", "."])
                .current_dir(&self.ic_repo_path)
                .stderr(Stdio::null())
                .stdout(Stdio::null())
                .status()
                .await?
                .success()
            {
                anyhow::bail!("Failed to clone ic repo")
            }

            return Ok(());
        }

        if !Command::new("git")
            .args(["switch", "master", "-f"])
            .current_dir(&self.ic_repo_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await?
            .success()
        {
            anyhow::bail!("Failed to switch branch to master")
        }

        if self.skip_pull {
            return Ok(());
        }

        if !Command::new("git")
            .args(["pull"])
            .current_dir(&self.ic_repo_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await?
            .success()
        {
            anyhow::bail!("Failed to pull master branch")
        }

        Ok(())
    }
}
