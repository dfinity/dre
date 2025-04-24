#[cfg(not(feature = "keyring"))]
use crate::pin::ask::AskEveryTimePinHandler;
#[cfg(feature = "keyring")]
use crate::pin::keyring::PlatformKeyringPinHandler;
use clap::Args as ClapArgs;
use clap_num::maybe_hex;
use dialoguer::{console::Term, theme::ColorfulTheme, Select};
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_canisters::parallel_hardware_identity::{hsm_key_id_to_int, KeyIdVec, ParallelHardwareIdentity};
use ic_canisters::IcAgentCanisterClient;
use ic_icrc1_test_utils::KeyPairGenerator;
use ic_management_types::Network;
use itertools::Itertools;
use log::{debug, info, warn};
use std::path::PathBuf;

/// HSM authentication parameters
#[derive(ClapArgs, Debug, Clone)]
pub(crate) struct HsmParams {
    /// Slot that HSM key uses, can be read with pkcs11-tool
    #[clap(required = false,
        conflicts_with = "private_key_pem",
        long, value_parser=maybe_hex::<u64>, global = true, env = "HSM_SLOT")]
    pub(crate) hsm_slot: Option<u64>,

    /// HSM Key ID, can be read with pkcs11-tool
    #[clap(required = false, conflicts_with = "private_key_pem", long, value_parser=maybe_hex::<u8>, global = true, env = "HSM_KEY_ID")]
    pub(crate) hsm_key_id: Option<KeyIdVec>,
}

/// HSM authentication arguments
/// These comprise an optional PIN and optional parameters.
/// The PIN is used during autodetection if the optional
/// parameters are missing.
#[derive(ClapArgs, Debug, Clone)]
pub(crate) struct HsmOpts {
    /// Pin for the HSM key used for submitting proposals
    // Must be present if slot and key are specified.
    #[clap(
        required = false,
        alias = "hsm-pim",
        conflicts_with = "private_key_pem",
        long,
        global = true,
        hide_env_values = true,
        env = "HSM_PIN"
    )]
    pub(crate) hsm_pin: Option<String>,
    #[clap(flatten)]
    pub(crate) hsm_params: HsmParams,
}

// The following should ideally be defined in terms of an Enum
// as there is no conceivable scenario in which both a PEM file
// and a set of HSM options can be used by the program.
// Sadly, until ticket
//   https://github.com/clap-rs/clap/issues/2621
// is fixed, we cannot do this, and we must use a struct instead.
// Note that group(multiple = false) has no effect, and therefore
// we have to use conflicts and requires to specify option deps.
#[derive(ClapArgs, Debug, Clone)]
#[group(multiple = false)]
/// Authentication arguments
pub struct AuthOpts {
    /// Path to private key file (in PEM format)
    #[clap(
        long,
        required = false,
        global = true,
        conflicts_with_all = ["hsm_pin", "hsm_slot", "hsm_key_id"],
        env = "PRIVATE_KEY_PEM",
        visible_aliases = &["pem", "key", "private-key"]
    )]
    pub(crate) private_key_pem: Option<String>,
    #[clap(flatten)]
    pub(crate) hsm_opts: HsmOpts,
}

impl TryFrom<String> for AuthOpts {
    type Error = anyhow::Error;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Ok(AuthOpts {
            private_key_pem: Some(value),
            hsm_opts: HsmOpts {
                hsm_pin: None,
                hsm_params: HsmParams {
                    hsm_slot: None,
                    hsm_key_id: None,
                },
            },
        })
    }
}

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

#[derive(Clone)]
pub enum AuthRequirement {
    Anonymous, // for get commands
    Signer,    // just authentication details used for signing
    Neuron,    // Signer + neuron_id used for proposals
}

pub const STAGING_NEURON_ID: u64 = 49;
pub const STAGING_KEY_PATH_FROM_HOME: &str = ".config/dfx/identity/bootstrap-super-leader/identity.pem";

impl Neuron {
    pub(crate) async fn from_opts_and_req(
        auth_opts: AuthOpts,
        requirement: AuthRequirement,
        network: &Network,
        neuron_id: Option<u64>,
        offline: bool,
        neuron_override: Option<Neuron>,
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

        let auth_specified = !matches!(
            auth_opts,
            AuthOpts {
                private_key_pem: None,
                hsm_opts: HsmOpts {
                    hsm_pin: None,
                    hsm_params: HsmParams {
                        hsm_slot: None,
                        hsm_key_id: None,
                    },
                },
            }
        );

        match requirement {
            AuthRequirement::Anonymous => Ok(Self {
                auth: Auth::Anonymous,
                neuron_id: 0,
                include_proposer: false,
            }),
            AuthRequirement::Signer => Ok(Self {
                // If nothing is specified for the signer and override is provided
                // use overide neuron for auth
                auth: match neuron_override {
                    Some(neuron) if !auth_specified => neuron.auth,
                    _ => Auth::from_auth_opts(auth_opts).await?,
                },
                neuron_id: 0,
                include_proposer: false,
            }),
            AuthRequirement::Neuron => Ok({
                if neuron_id.is_none() && !auth_specified && neuron_override.is_some() {
                    info!("Using override neuron for this command since no auth options were provided");
                    neuron_override.unwrap()
                } else {
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
            fs_err::create_dir_all(parent)?
        }

        let key_pair = rosetta_core::models::Ed25519KeyPair::generate(42);

        if !path.exists() {
            fs_err::write(&path, key_pair.to_pem())?;
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

    pub fn maybe_proposer(&self) -> Option<String> {
        if self.include_proposer {
            Some(self.neuron_id.to_string())
        } else {
            None
        }
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

const AUTOMATION_NEURON_DEFAULT_PATH: &str = ".config/dfx/identity/release-automation/identity.pem";
pub fn get_automation_neuron_default_path() -> PathBuf {
    let home = dirs::home_dir().unwrap();
    home.join(AUTOMATION_NEURON_DEFAULT_PATH)
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
                hsm_key_id_to_int(&identity.key_id),
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
        let neuron_ids = response.neuron_infos.keys().copied().sorted().collect::<Vec<_>>();
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

    /// FIXME: this should not return anyhow::Error, but rather a structured error,
    /// since anyhow swallows panics, and transforms them to errors.
    fn detect_hsm_auth(maybe_pin: Option<String>, maybe_slot: Option<u64>, maybe_key_id: Option<KeyIdVec>) -> anyhow::Result<Self> {
        #[cfg(feature = "keyring")]
        let memory = PlatformKeyringPinHandler::default();
        #[cfg(not(feature = "keyring"))]
        let memory = AskEveryTimePinHandler::default();
        Ok(Auth::Hsm {
            identity: ParallelHardwareIdentity::scan(maybe_pin, maybe_slot, maybe_key_id, &memory)?,
        })
    }

    /// Create an Auth that automatically detects an HSM.  Falls back to
    /// anonymous authentication if no HSM is detected.  Prompts the user
    /// for a PIN if no PIN is specified and the HSM needs to be unlocked.
    /// Caller can optionally limit search to a specific slot or key ID.
    async fn auto(hsm_pin: Option<String>, hsm_slot: Option<u64>, hsm_key_id: Option<KeyIdVec>) -> anyhow::Result<Self> {
        tokio::task::spawn_blocking(move || Self::detect_hsm_auth(hsm_pin, hsm_slot, hsm_key_id)).await?
    }

    /// Create an Auth that uses a specified PEM file.
    async fn pem(private_key_pem: PathBuf) -> anyhow::Result<Self> {
        // Check path exists.
        if !private_key_pem.exists() {
            return Err(anyhow::anyhow!("Private key file not found: {:?}", private_key_pem));
        }
        Ok(Self::Keyfile { path: private_key_pem })
    }

    /// Create an Auth based on the specified user options.
    pub(crate) async fn from_auth_opts(auth_opts: AuthOpts) -> Result<Self, anyhow::Error> {
        match auth_opts {
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
            } => Auth::auto(pin, hsm_slot, hsm_key_id).await,
        }
    }
}
