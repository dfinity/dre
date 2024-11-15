use std::path::PathBuf;

use anyhow::anyhow;
use dialoguer::{console::Term, theme::ColorfulTheme, Password, Select};
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_canisters::parallel_hardware_identity::{hsm_key_id_to_string, KeyIdVec, ParallelHardwareIdentity};
use ic_canisters::IcAgentCanisterClient;
use ic_icrc1_test_utils::KeyPairGenerator;
use ic_management_types::Network;
use keyring::{Entry, Error};
use log::{debug, info, warn};

use crate::commands::{AuthOpts, AuthRequirement, HsmOpts, HsmParams};

#[derive(Debug, Clone)]
pub struct Neuron {
    pub auth: Auth,
    pub neuron_id: u64,
    pub include_proposer: bool,
}

impl PartialEq for Neuron {
    fn eq(&self, other: &Self) -> bool {
        self.neuron_id == other.neuron_id && self.include_proposer == other.include_proposer
    }
}

impl Eq for Neuron {}

pub const STAGING_NEURON_ID: u64 = 49;
pub const STAGING_KEY_PATH_FROM_HOME: &str = ".config/dfx/identity/bootstrap-super-leader/identity.pem";

impl Neuron {
    pub(crate) async fn from_opts_and_req(
        auth_opts: AuthOpts,
        requirement: AuthRequirement,
        network: &Network,
        neuron_id: Option<u64>,
        offline: bool,
    ) -> anyhow::Result<Self> {
        let (neuron_id, auth_opts) = if network.name == "staging" {
            let staging_known_path = dirs::home_dir().expect("Home dir should be set").join(STAGING_KEY_PATH_FROM_HOME);

            match neuron_id {
                Some(n) => (Some(n), auth_opts),
                None => (
                    Some(STAGING_NEURON_ID),
                    match Auth::pem(staging_known_path.clone()).await {
                        Ok(_) => AuthOpts {
                            private_key_pem: Some(staging_known_path.display().to_string()),
                            hsm_opts: HsmOpts {
                                hsm_pin: None,
                                hsm_params: HsmParams {
                                    hsm_slot: None,
                                    hsm_key_id: None,
                                },
                            },
                        },
                        Err(e) => match requirement {
                            // If there is an error but auth is not needed
                            // just send what we have since it won't be
                            // checked anyway
                            AuthRequirement::Anonymous => auth_opts,
                            _ => anyhow::bail!("Failed to find staging auth: {:?}", e),
                        },
                    },
                ),
            }
        } else {
            (neuron_id, auth_opts)
        };

        match requirement {
            AuthRequirement::Anonymous => Ok(Self {
                auth: Auth::Anonymous,
                neuron_id: 0,
                include_proposer: false,
            }),
            AuthRequirement::Signer => Ok(Self {
                auth: Auth::from_auth_opts(auth_opts).await?,
                neuron_id: 0,
                include_proposer: false,
            }),
            AuthRequirement::Neuron => Ok({
                match (neuron_id, offline) {
                    (Some(n), _) => Self {
                        neuron_id: n,
                        auth: Auth::from_auth_opts(auth_opts).await?,
                        include_proposer: true,
                    },
                    // This is just a placeholder since
                    // the tool is instructed to run in
                    // offline mode.
                    (None, true) => {
                        warn!("Required full neuron but offline mode instructed! Will not attempt to auto-detect neuron id");
                        Self {
                            neuron_id: 0,
                            auth: Auth::from_auth_opts(auth_opts).await?,
                            include_proposer: true,
                        }
                    }
                    (None, false) => {
                        let auth = Auth::from_auth_opts(auth_opts).await?;
                        let neuron_id = auth.clone().auto_detect_neuron_id(network.nns_urls.clone()).await?;
                        Self {
                            neuron_id,
                            auth,
                            include_proposer: true,
                        }
                    }
                }
            }),
        }
    }

    #[cfg(test)]
    pub fn ensure_fake_pem_outer(name: &str) -> anyhow::Result<PathBuf> {
        Self::ensure_fake_pem(name)
    }

    fn ensure_fake_pem(name: &str) -> anyhow::Result<PathBuf> {
        let home_dir = dirs::home_dir().ok_or(anyhow::anyhow!("Home dir not set"))?;
        let path = home_dir.join(format!(".config/dfx/identity/{}/identity.pem", name));

        let parent = path.parent().ok_or(anyhow::anyhow!("Expected parent to exist"))?;
        if !parent.exists() {
            std::fs::create_dir_all(parent)?
        }

        let key_pair = rosetta_core::models::Ed25519KeyPair::generate(42);

        if !path.exists() {
            std::fs::write(&path, key_pair.to_pem())?;
        }
        Ok(path)
    }

    pub fn dry_run_fake_neuron() -> anyhow::Result<Self> {
        Ok(Self {
            auth: Auth::Keyfile {
                path: Self::ensure_fake_pem("test_neuron_1")?,
            },
            include_proposer: true,
            neuron_id: 123,
        })
    }

    pub fn as_arg_vec(&self) -> Vec<String> {
        self.auth.as_arg_vec()
    }

    pub fn proposer_as_arg_vec(&self) -> Vec<String> {
        if self.include_proposer {
            return vec!["--proposer".to_string(), self.neuron_id.to_string()];
        }
        vec![]
    }

    pub fn anonymous_neuron() -> Self {
        debug!("Constructing anonymous neuron (ID 0)");
        Self {
            auth: Auth::Anonymous,
            neuron_id: 0,
            include_proposer: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Auth {
    Hsm { identity: ParallelHardwareIdentity },
    Keyfile { path: PathBuf },
    Anonymous,
}

impl Auth {
    pub fn as_arg_vec(&self) -> Vec<String> {
        match self {
            Auth::Hsm { identity } => vec![
                "--use-hsm".to_string(),
                "--pin".to_string(),
                identity.cached_pin.clone(),
                "--slot".to_string(),
                identity.slot.to_string(),
                "--key-id".to_string(),
                hsm_key_id_to_string(&identity.key_id),
            ],
            Auth::Keyfile { path } => vec!["--secret-key-pem".to_string(), path.to_string_lossy().to_string()],
            Auth::Anonymous => vec![],
        }
    }

    pub fn create_canister_client(self, nns_urls: Vec<url::Url>) -> anyhow::Result<IcAgentCanisterClient> {
        // FIXME: why do we even take multiple URLs if only the first one is ever used?
        let url = nns_urls.first().ok_or(anyhow::anyhow!("No NNS URLs provided"))?.to_owned();
        match self {
            Auth::Hsm { identity } => IcAgentCanisterClient::from_hsm(identity, url),
            Auth::Keyfile { path } => IcAgentCanisterClient::from_key_file(path.clone(), url),
            Auth::Anonymous => IcAgentCanisterClient::from_anonymous(url),
        }
    }

    async fn auto_detect_neuron_id(self, nns_urls: Vec<url::Url>) -> anyhow::Result<u64> {
        let nnsurlsclone = nns_urls.clone();
        let client = tokio::task::spawn_blocking(move || self.create_canister_client(nnsurlsclone)).await??;
        let governance = GovernanceCanisterWrapper::from(client);
        let response = governance.list_neurons().await?;
        let neuron_ids = response.neuron_infos.keys().copied().collect::<Vec<_>>();
        match neuron_ids.len() {
            0 => Err(anyhow::anyhow!(
                "Hardware security module doesn't control any neurons. Response from governance canister: {:?}",
                response
            )),
            1 => Ok(neuron_ids[0]),
            _ => Select::with_theme(&ColorfulTheme::default())
                .items(&neuron_ids)
                .default(0)
                .interact_on_opt(&Term::stderr())?
                .map(|i| neuron_ids[i])
                .ok_or_else(|| anyhow::anyhow!("No neuron selected")),
        }
    }

    /// If it is called it is expected to retrieve an Auth of type Hsm or fail
    /// FIXME: this should not return anyhow::Error, but rather a structured error,
    /// since anyhow swallows panics, and transforms them to errors.
    fn detect_hsm_auth(maybe_pin: Option<String>, maybe_slot: Option<u64>, maybe_key_id: Option<KeyIdVec>) -> anyhow::Result<Self> {
        let detected_hsm = ParallelHardwareIdentity::scan_for_hsm(maybe_slot, maybe_key_id)?;
        let identity = match maybe_pin {
            Some(pin) => detected_hsm.authenticate(pin),
            None => {
                let pin_entry = Entry::new("dre-tool-hsm-pin", detected_hsm.memo_key.as_str())?;
                let tentative_pin = match pin_entry.get_password() {
                    Err(Error::NoEntry) => Password::new()
                        .with_prompt("Please enter the hardware security module PIN: ")
                        .interact()?,
                    Ok(pin) => pin,
                    Err(e) => return Err(anyhow!("Problem getting PIN from keyring: {}", e)),
                };
                match detected_hsm.authenticate(tentative_pin.clone()) {
                    Ok(identity) => {
                        pin_entry.set_password(&tentative_pin)?;
                        Ok(identity)
                    }
                    Err(e) => {
                        pin_entry.delete_credential()?;
                        return Err(anyhow!("Hardware security module PIN incorrect ({})", e));
                    }
                }
            }
        }?;
        Ok(Auth::Hsm { identity })
    }

    /// Create an Auth that automatically detects an HSM.  Falls back to
    /// anonymous authentication if no HSM is detected.  Prompts the user
    /// for a PIN if no PIN is specified and the HSM needs to be unlocked.
    /// Caller can optionally limit search to a specific slot or key ID.
    pub async fn auto(hsm_pin: Option<String>, hsm_slot: Option<u64>, hsm_key_id: Option<KeyIdVec>) -> anyhow::Result<Self> {
        tokio::task::spawn_blocking(move || Self::detect_hsm_auth(hsm_pin, hsm_slot, hsm_key_id)).await?
    }

    pub async fn pem(private_key_pem: PathBuf) -> anyhow::Result<Self> {
        // Check path exists.
        if !private_key_pem.exists() {
            return Err(anyhow::anyhow!("Private key file not found: {:?}", private_key_pem));
        }
        Ok(Self::Keyfile { path: private_key_pem })
    }

    pub(crate) async fn from_auth_opts(auth_opts: AuthOpts) -> Result<Self, anyhow::Error> {
        match &auth_opts {
            // Private key case.
            AuthOpts {
                private_key_pem: Some(private_key_pem),
                hsm_opts: _,
            } => {
                info!("Using requested private key file {}", private_key_pem);
                Auth::pem(PathBuf::from(private_key_pem)).await
            }
            // Slot and key case.
            // Also autodetect case.
            AuthOpts {
                private_key_pem: _,
                hsm_opts:
                    HsmOpts {
                        hsm_pin: pin,
                        hsm_params: HsmParams { hsm_slot, hsm_key_id },
                    },
            } => Auth::auto(pin.clone(), *hsm_slot, hsm_key_id.clone()).await,
        }
    }
}
