use std::{path::PathBuf, str::FromStr};

use clap::Parser;

use ic_nervous_system_common_test_keys::TEST_NEURON_1_OWNER_KEYPAIR;
const TEST_NEURON_1_IDENTITY_PATH: &str = ".config/dfx/identity/test_neuron_1/identity.pem";

#[derive(Parser)]
#[clap(about, version)]
pub struct Args {
    /// Version to qualify
    pub version_to_qualify: String,

    /// Specify a version from which the qualification
    /// should start. The default will be the same
    /// version as the NNS
    #[clap(long)]
    pub initial_version: Option<String>,

    /// Path which contains the layout of the network to
    /// be deployed. The default value will be a network
    /// consisting of:
    ///   2 application subnets (4 nodes per subnet)
    ///   1 system subnet (4 nodes)
    ///   4 unassigned nodes
    #[clap(long)]
    pub config_override: Option<PathBuf>,
}

impl Args {
    pub fn ensure_key(&self) -> anyhow::Result<()> {
        let key_pair = &TEST_NEURON_1_OWNER_KEYPAIR;
        let path = dirs::home_dir()
            .ok_or(anyhow::anyhow!("No home dir present"))?
            .join(PathBuf::from_str(TEST_NEURON_1_IDENTITY_PATH)?);
        let dir = path.parent().ok_or(anyhow::anyhow!("No parent dir for path: {}", path.display()))?;
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
        }

        std::fs::write(path, key_pair.to_pem()).map_err(|e| anyhow::anyhow!(e))
    }
}
