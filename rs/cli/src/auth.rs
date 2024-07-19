use std::path::PathBuf;
use std::{fs::read_to_string, str::FromStr};

use candid::{Decode, Encode};
use cryptoki::{
    context::{CInitializeArgs, Pkcs11},
    session::{SessionFlags, UserType},
};
use dialoguer::{console::Term, theme::ColorfulTheme, Password, Select};
use ic_canister_client::{Agent, Sender};
use ic_canister_client_sender::SigKeys;
use ic_management_types::Network;
use ic_nns_constants::GOVERNANCE_CANISTER_ID;
use ic_nns_governance::pb::v1::{ListNeurons, ListNeuronsResponse};
use ic_sys::utility_command::UtilityCommand;
use keyring::{Entry, Error};
use log::info;

#[derive(Clone, Debug)]
pub struct Neuron {
    pub auth: Auth,
    pub neuron_id: u64,
    pub include_proposer: bool,
}

static RELEASE_AUTOMATION_DEFAULT_PRIVATE_KEY_PEM: &str = ".config/dfx/identity/release-automation/identity.pem"; // Relative to the home directory
const RELEASE_AUTOMATION_NEURON_ID: u64 = 80;

impl Neuron {
    pub async fn new(
        private_key_pem: Option<PathBuf>,
        hsm_slot: Option<u64>,
        hsm_pin: Option<String>,
        hsm_key_id: Option<String>,
        neuron_id: Option<u64>,
        network: &Network,
        include_proposer: bool,
    ) -> anyhow::Result<Self> {
        let auth = Auth::from_cli_args(private_key_pem, hsm_slot, hsm_pin, hsm_key_id)?;
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

    pub fn from_cli_args(
        private_key_pem: Option<PathBuf>,
        hsm_slot: Option<u64>,
        hsm_pin: Option<String>,
        hsm_key_id: Option<String>,
    ) -> anyhow::Result<Self> {
        match (private_key_pem, hsm_slot, hsm_pin, hsm_key_id) {
            (Some(path), _, _, _) if path.exists() => Ok(Auth::Keyfile { path }),
            (Some(path), _, _, _) => Err(anyhow::anyhow!("Invalid key file path: {}", path.display())),
            (None, Some(slot), Some(pin), Some(key_id)) => Ok(Auth::Hsm { pin, slot, key_id }),
            (None, None, None, None) => Ok(Self::detect_hsm_auth()?.map_or(Auth::Anonymous, |a| a)),
            _ => Err(anyhow::anyhow!("Invalid auth arguments")),
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
        let sender = match self {
            Auth::Hsm { pin, slot, key_id } => {
                let pin_clone = pin.clone();
                let slot = *slot;
                let key_id_clone = key_id.clone();
                Sender::from_external_hsm(
                    UtilityCommand::read_public_key(Some(&slot.to_string()), Some(&key_id_clone)).execute()?,
                    std::sync::Arc::new(move |input| {
                        Ok(UtilityCommand::sign_message(input.to_vec(), Some(&slot.to_string()), Some(&pin_clone), Some(&key_id_clone)).execute()?)
                    }),
                )
            }
            Auth::Keyfile { path } => {
                let contents = read_to_string(path).expect("Could not read key file");
                let sig_keys = SigKeys::from_pem(&contents).expect("Failed to parse pem file");
                Sender::SigKeys(sig_keys)
            }
            Auth::Anonymous => Sender::Anonymous,
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
                    "HSM doesn't control any neurons. Response from governance canister: {:?}",
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
}
