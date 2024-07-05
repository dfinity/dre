use std::{cell::RefCell, path::PathBuf, rc::Rc, str::FromStr};

use ic_canisters::governance::governance_canister_version;
use ic_management_types::Network;

use crate::commands::Args;

const STAGING_NEURON_ID: u64 = 49;
pub struct DreContext {
    network: Network,
    private_key_pem: Option<PathBuf>,
    neuron: Option<u64>,
    governance_canister_version_hash: String,
}

impl DreContext {
    pub async fn from_args(args: Args) -> anyhow::Result<Self> {
        let network = ic_management_types::Network::new(args.network.clone(), &args.nns_urls)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        let nns_urls = network.get_nns_urls();

        let private_key_pem = match args.private_key_pem {
            Some(path) => Some(PathBuf::from(path)),
            None if network.name == "staging" => {
                let path = PathBuf::from_str(&std::env::var("HOME")?)?.join("/.config/dfx/identity/bootstrap-super-leader/identity.pem");
                match path.exists() {
                    true => Some(path),
                    false => None,
                }
            }
            None => None,
        };

        let govn_canister_version = governance_canister_version(nns_urls).await.map_err(|e| anyhow::anyhow!(e))?;

        Ok(Self {
            network,
            private_key_pem,
            neuron: args.neuron_id,
            governance_canister_version_hash: govn_canister_version.stringified_hash,
        })
    }
}
