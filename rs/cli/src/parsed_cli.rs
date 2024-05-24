use ic_management_types::{Artifact, Network};

use crate::{cli::Opts, detect_neuron::Neuron};

#[derive(Clone)]
pub struct ParsedCli {
    pub network: Network,
    pub ic_admin_bin_path: Option<String>,
    pub yes: bool,
    pub neuron: Neuron,
}

#[derive(Clone)]
pub struct UpdateVersion {
    pub release_artifact: Artifact,
    pub version: String,
    pub title: String,
    pub summary: String,
    pub update_urls: Vec<String>,
    pub stringified_hash: String,
    pub versions_to_retire: Option<Vec<String>>,
}

impl ParsedCli {
    pub fn get_neuron(&self) -> &Neuron {
        &self.neuron
    }

    pub fn get_update_cmd_args(update_version: &UpdateVersion) -> Vec<String> {
        [
            [
                vec![
                    format!("--{}-version-to-elect", update_version.release_artifact),
                    update_version.version.to_string(),
                    "--release-package-sha256-hex".to_string(),
                    update_version.stringified_hash.to_string(),
                    "--release-package-urls".to_string(),
                ],
                update_version.update_urls.clone(),
            ]
            .concat(),
            match update_version.versions_to_retire.clone() {
                Some(versions) => [vec![format!("--{}-versions-to-unelect", update_version.release_artifact)], versions].concat(),
                None => vec![],
            },
        ]
        .concat()
    }

    pub async fn from_opts(opts: &Opts) -> anyhow::Result<Self> {
        let network = Network::new(&opts.network, &opts.nns_urls).await.map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse network from name {} and NNS urls {:?}. Error: {}",
                opts.network,
                opts.nns_urls,
                e
            )
        })?;
        let neuron = Neuron::new(
            &network,
            opts.neuron_id,
            opts.private_key_pem.clone(),
            opts.hsm_slot,
            opts.hsm_pin.clone(),
            opts.hsm_key_id.clone(),
        )
        .await;
        Ok(ParsedCli {
            network,
            yes: opts.yes,
            neuron,
            ic_admin_bin_path: opts.ic_admin.clone(),
        })
    }
}
