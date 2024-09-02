use std::path::PathBuf;
use std::str::FromStr;

use cryptoki::{
    context::{CInitializeArgs, Pkcs11},
    session::{SessionFlags, UserType},
};
use dialoguer::{console::Term, theme::ColorfulTheme, Password, Select};
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_canisters::IcAgentCanisterClient;
use ic_management_types::Network;
use keyring::{Entry, Error};
use log::info;

use crate::commands::AuthOpts;

#[derive(Clone, Debug)]
pub struct Neuron {
    pub auth: Auth,
    pub neuron_id: u64,
    pub include_proposer: bool,
}

static RELEASE_AUTOMATION_DEFAULT_PRIVATE_KEY_PEM: &str = ".config/dfx/identity/release-automation/identity.pem"; // Relative to the home directory
const RELEASE_AUTOMATION_NEURON_ID: u64 = 80;

impl Neuron {
    pub async fn new(auth_opts: AuthOpts, neuron_id: Option<u64>, network: &Network, include_proposer: bool) -> anyhow::Result<Self> {
        let auth = Auth::try_from(auth_opts)?;
        let neuron_id = match neuron_id {
            Some(n) => n,
            None => auth.auto_detect_neuron_id(network.get_nns_urls()).await?,
        };
        Ok(Self {
            auth,
            neuron_id,
            include_proposer,
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

    pub fn automation_neuron_unchecked() -> Self {
        Self {
            auth: Auth::Keyfile {
                path: dirs::home_dir().unwrap().join(RELEASE_AUTOMATION_DEFAULT_PRIVATE_KEY_PEM),
            },
            neuron_id: RELEASE_AUTOMATION_NEURON_ID,
            include_proposer: true,
        }
    }

    pub fn anonymous_neuron() -> Self {
        Self {
            auth: Auth::Anonymous,
            neuron_id: 0,
            include_proposer: false,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Auth {
    Hsm { pin: String, slot: u64, key_id: String },
    Keyfile { path: PathBuf },
    Anonymous,
}

impl Auth {
    pub fn as_arg_vec(&self) -> Vec<String> {
        match self {
            Auth::Hsm { pin, slot, key_id } => vec![
                "--use-hsm".to_string(),
                "--pin".to_string(),
                pin.clone(),
                "--slot".to_string(),
                slot.to_string(),
                "--key-id".to_string(),
                key_id.clone(),
            ],
            Auth::Keyfile { path } => vec!["--secret-key-pem".to_string(), path.to_string_lossy().to_string()],
            Auth::Anonymous => vec![],
        }
    }

    fn pkcs11_lib_path() -> anyhow::Result<PathBuf> {
        let lib_macos_path = PathBuf::from_str("/Library/OpenSC/lib/opensc-pkcs11.so")?;
        let lib_linux_path = PathBuf::from_str("/usr/lib/x86_64-linux-gnu/opensc-pkcs11.so")?;
        if lib_macos_path.exists() {
            Ok(lib_macos_path)
        } else if lib_linux_path.exists() {
            Ok(lib_linux_path)
        } else {
            Err(anyhow::anyhow!("no pkcs11 library found"))
        }
    }

    fn detect_hsm_auth() -> anyhow::Result<Option<Self>> {
        info!("Detecting HSM devices");
        let ctx = Pkcs11::new(Self::pkcs11_lib_path()?)?;
        ctx.initialize(CInitializeArgs::OsThreads)?;
        for slot in ctx.get_slots_with_token()? {
            let info = ctx.get_slot_info(slot)?;
            if info.slot_description().starts_with("Nitrokey Nitrokey HSM") {
                let key_id = format!("hsm-{}-{}", info.slot_description(), info.manufacturer_id());
                let pin_entry = Entry::new("dre-tool-hsm-pin", &key_id)?;
                let pin = match pin_entry.get_password() {
                    // TODO: Remove the old keyring entry search ("release-cli") after August 1st, 2024
                    Err(Error::NoEntry) => match Entry::new("release-cli", &key_id) {
                        Err(Error::NoEntry) => Password::new().with_prompt("Please enter the HSM PIN: ").interact()?,
                        Ok(pin_entry) => pin_entry.get_password()?,
                        Err(e) => return Err(anyhow::anyhow!("Failed to get pin from keyring: {}", e)),
                    },
                    Ok(pin) => pin,
                    Err(e) => return Err(anyhow::anyhow!("Failed to get pin from keyring: {}", e)),
                };

                let mut flags = SessionFlags::new();
                flags.set_serial_session(true);
                let session = ctx.open_session_no_callback(slot, flags).unwrap();
                session.login(UserType::User, Some(&pin))?;
                info!("HSM login successful!");
                pin_entry.set_password(&pin)?;
                return Ok(Some(Auth::Hsm {
                    pin,
                    slot: slot.id(),
                    key_id: "01".to_string(),
                }));
            }
        }
        Ok(None)
    }

    pub async fn auto_detect_neuron_id(&self, nns_urls: &[url::Url]) -> anyhow::Result<u64> {
        let url = nns_urls.first().ok_or(anyhow::anyhow!("No nns urls provided"))?.to_owned();
        let client = match self {
            Auth::Hsm { pin, slot, key_id } => {
                let pin_clone = pin.clone();
                let slot = *slot;
                let key_id_clone = key_id.clone();
                IcAgentCanisterClient::from_hsm(pin_clone, slot, key_id_clone, url, None)?
            }
            Auth::Keyfile { path } => IcAgentCanisterClient::from_key_file(path.clone(), url)?,
            Auth::Anonymous => IcAgentCanisterClient::from_anonymous(url)?,
        };
        let governance = GovernanceCanisterWrapper::from(client);
        let response = governance.list_neurons().await?;
        let neuron_ids = response.neuron_infos.keys().copied().collect::<Vec<_>>();
        match neuron_ids.len() {
            0 => Err(anyhow::anyhow!(
                "HSM doesn't control any neurons. Response fro governance canister: {:?}",
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
}

impl TryFrom<AuthOpts> for Auth {
    type Error = anyhow::Error;
    fn try_from(auth_opts: AuthOpts) -> Result<Self, anyhow::Error> {
        match &auth_opts {
            AuthOpts {
                private_key_pem: Some(private_key_pem),
                hsm_opts: _,
            } => Ok(Auth::Keyfile {
                path: private_key_pem.path().to_path_buf(),
            }),
            AuthOpts {
                private_key_pem: _,
                hsm_opts: Some(hsm_opts),
            } => Ok(Auth::Hsm {
                pin: hsm_opts.hsm_pin.clone(),
                slot: hsm_opts.hsm_slot,
                key_id: hsm_opts.hsm_key_id.clone(),
            }),
            // I think the next line should not fall back to anonymous.
            // It should always be autodetect and fail if detection fails.
            _ => Ok(Self::detect_hsm_auth()?.map_or(Auth::Anonymous, |a| a)),
        }
    }
}
