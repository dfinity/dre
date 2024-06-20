use candid::{Decode, Encode};
use cryptoki::{
    context::{CInitializeArgs, Pkcs11},
    session::{SessionFlags, UserType},
};
use dialoguer::{console::Term, theme::ColorfulTheme, Password, Select};
use ic_canister_client::{Agent, Sender};
use ic_canister_client_sender::SigKeys;
use ic_nns_constants::GOVERNANCE_CANISTER_ID;
use ic_nns_governance::pb::v1::{ListNeurons, ListNeuronsResponse};
use ic_sys::utility_command::UtilityCommand;
use keyring::{Entry, Error};
use log::{info, warn};
use std::{cell::RefCell, fs::read_to_string, path::PathBuf, str::FromStr};

static RELEASE_AUTOMATION_DEFAULT_PRIVATE_KEY_PEM: &str = ".config/dfx/identity/release-automation/identity.pem"; // Relative to the home directory
const RELEASE_AUTOMATION_NEURON_ID: u64 = 80;

#[derive(Clone, Debug)]
pub struct Neuron {
    network: ic_management_types::Network,
    neuron_id: RefCell<Option<u64>>,
    private_key_pem: Option<PathBuf>,
    hsm_slot: Option<u64>,
    hsm_pin: Option<String>,
    hsm_key_id: Option<String>,
    auth_cache: RefCell<Option<Auth>>,
}

impl Neuron {
    pub async fn new(
        network: &ic_management_types::Network,
        neuron_id: Option<u64>,
        private_key_pem: Option<String>,
        hsm_slot: Option<u64>,
        hsm_pin: Option<String>,
        hsm_key_id: Option<String>,
    ) -> Self {
        let private_key_pem = match private_key_pem {
            Some(path) => match PathBuf::from_str(&path).expect("Cannot parse the private key path") {
                path if path.exists() => Some(path),
                _ => {
                    warn!("Invalid private key path");
                    None
                }
            },
            None => None,
        };
        Self {
            network: network.clone(),
            neuron_id: RefCell::new(neuron_id),
            private_key_pem,
            hsm_slot,
            hsm_pin,
            hsm_key_id,
            auth_cache: RefCell::new(None),
        }
    }

    pub async fn get_auth(&self, allow_auth: bool) -> anyhow::Result<Auth> {
        if !allow_auth {
            // This is used for the `get-*` commands, which don't accept authentification parameters
            return Ok(Auth::None);
        }

        if let Some(auth) = &*self.auth_cache.borrow() {
            return Ok(auth.clone());
        };

        let auth = if let Some(path) = &self.private_key_pem {
            Auth::Keyfile { path: path.clone() }
        } else {
            // If HSM slot, pin and key id are provided, use them without checking
            if let (Some(slot), Some(pin), Some(key_id)) = (self.hsm_slot, &self.hsm_pin, &self.hsm_key_id) {
                Auth::Hsm {
                    pin: pin.clone(),
                    slot,
                    key_id: key_id.clone(),
                }
            } else {
                // Fully automatic detection of the neuron id using HSM
                match detect_hsm_auth() {
                    Ok(Some(auth)) => auth,
                    Ok(None) => Auth::None,
                    Err(e) => {
                        warn!("Failed to detect HSM: {}", e);
                        Auth::None
                    }
                }
            }
        };
        self.auth_cache.borrow_mut().get_or_insert_with(|| auth.clone());
        Ok(auth)
    }

    pub async fn get_neuron_id(&self) -> anyhow::Result<u64> {
        if let Some(neuron_id) = *self.neuron_id.borrow() {
            return Ok(neuron_id);
        };
        let neuron_id = auto_detect_neuron_id(self.network.get_nns_urls(), self.get_auth(true).await?).await?;
        self.neuron_id.replace(Some(neuron_id));
        Ok(neuron_id)
    }

    /// Returns the arguments to pass to the ic-admin CLI for this neuron.
    /// If require_auth is true, it will panic if the auth method could not be detected.
    /// if allow_auth is false, it will return an empty vector in any case.
    /// if allow_auth is true, it will try to detect the auth parameters: method and neuron id.
    /// Detection of the auth parameters is useful to check if the auth detection works correctly without
    /// actually submitting a proposal.
    pub async fn as_arg_vec(&self, require_auth: bool, allow_auth: bool) -> anyhow::Result<Vec<String>> {
        // If auth may be provided (allow_auth), search for valid neuron id using HSM or with the private key
        // `allow_auth` is set to false for ic-admin `get-*` commands, since they don't accept auth
        // If private key is provided, use it without checking
        let auth = if allow_auth {
            match self.get_auth(allow_auth).await {
                Ok(auth) => auth,
                Err(e) => {
                    if require_auth {
                        return Err(anyhow::anyhow!(e));
                    } else {
                        return Ok(vec![]);
                    }
                }
            }
        } else {
            Auth::None
        };
        let neuron_id = match auto_detect_neuron_id(self.network.get_nns_urls(), auth).await {
            Ok(neuron_id) => neuron_id,
            Err(e) => {
                if require_auth {
                    return Err(anyhow::anyhow!(e));
                } else {
                    return Ok(vec![]);
                }
            }
        };
        Ok(vec!["--proposer".to_string(), neuron_id.to_string()])
    }

    pub fn as_automation(self) -> Self {
        let private_key_pem = match self.private_key_pem {
            Some(private_key_pem) => private_key_pem,
            None => dirs::home_dir()
                .expect("failed to find the home dir")
                .join(RELEASE_AUTOMATION_DEFAULT_PRIVATE_KEY_PEM),
        };

        Self {
            private_key_pem: Some(private_key_pem),
            neuron_id: RefCell::new(Some(RELEASE_AUTOMATION_NEURON_ID)),
            ..self
        }
    }
}

#[derive(Clone, Debug)]
pub enum Auth {
    Hsm { pin: String, slot: u64, key_id: String },
    Keyfile { path: PathBuf },
    None,
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

pub fn get_pkcs11_ctx() -> anyhow::Result<Pkcs11> {
    let pkcs11 = Pkcs11::new(pkcs11_lib_path()?)?;
    pkcs11.initialize(CInitializeArgs::OsThreads)?;
    Ok(pkcs11)
}

impl Auth {
    /// Returns the arguments to pass to the ic-admin CLI for the given auth method
    /// If require_auth is true, it will panic if the auth method is Auth::None
    /// If allow_auth is false, it will return an empty vector in any case.
    /// If allow_auth is true, it will return the arguments for the set auth method.
    /// Checking the auth parameters is useful to validate working auth detection.
    pub fn as_arg_vec(&self, require_auth: bool, allow_auth: bool) -> Vec<String> {
        if !allow_auth {
            return vec![];
        }
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
            Auth::None => {
                if require_auth {
                    panic!("Auth required")
                } else {
                    vec![]
                }
            }
        }
    }

    pub fn from_cli_args(
        private_key_pem: Option<String>,
        hsm_slot: Option<u64>,
        hsm_pin: Option<String>,
        hsm_key_id: Option<String>,
    ) -> anyhow::Result<Self> {
        match (private_key_pem, hsm_slot, hsm_pin, hsm_key_id) {
            (Some(path), _, _, _) if PathBuf::from(path.clone()).exists() => Ok(Auth::Keyfile { path: PathBuf::from(path) }),
            (Some(path), _, _, _) => Err(anyhow::anyhow!("Invalid key file path: {}", path)),
            (None, Some(slot), Some(pin), Some(key_id)) => Ok(Auth::Hsm { pin, slot, key_id }),
            _ => Err(anyhow::anyhow!("Invalid auth arguments")),
        }
    }
}

pub fn detect_hsm_auth() -> anyhow::Result<Option<Auth>> {
    info!("Detecting HSM devices");
    let ctx = get_pkcs11_ctx()?;
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

pub async fn auto_detect_neuron_id(nns_urls: &[url::Url], auth: Auth) -> anyhow::Result<u64> {
    let sender = match auth {
        Auth::Hsm { pin, slot, key_id } => Sender::from_external_hsm(
            UtilityCommand::read_public_key(Some(&slot.to_string()), Some(&key_id)).execute()?,
            std::sync::Arc::new(move |input| {
                Ok(UtilityCommand::sign_message(input.to_vec(), Some(&slot.to_string()), Some(&pin), Some(&key_id)).execute()?)
            }),
        ),
        Auth::Keyfile { path } => {
            let contents = read_to_string(path).expect("Could not read key file");
            let sig_keys = SigKeys::from_pem(&contents).expect("Failed to parse pem file");
            Sender::SigKeys(sig_keys)
        }
        Auth::None => return Err(anyhow::anyhow!("No auth provided")),
    };
    let agent = Agent::new(nns_urls[0].clone(), sender);
    if let Some(response) = agent
        .execute_query(
            &GOVERNANCE_CANISTER_ID,
            "list_neurons",
            Encode!(&ListNeurons {
                include_neurons_readable_by_caller: true,
                neuron_ids: vec![],
            })?,
        )
        .await
        .map_err(|e| anyhow::anyhow!(e))?
    {
        let response = Decode!(&response, ListNeuronsResponse)?;
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
    } else {
        Err(anyhow::anyhow!("Empty response when listing controlled neurons"))
    }
}
