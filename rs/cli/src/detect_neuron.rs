use std::{path::PathBuf, str::FromStr};

use anyhow::Context;
use candid::{Decode, Encode};
use cryptoki::{
    context::{CInitializeArgs, Pkcs11},
    session::{SessionFlags, UserType},
};
use dialoguer::{console::Term, theme::ColorfulTheme, Password, Select};
use ic_canister_client::{Agent, Sender};
use ic_nns_constants::GOVERNANCE_CANISTER_ID;
use ic_nns_governance::pb::v1::{ListNeurons, ListNeuronsResponse};
use ic_sys::utility_command::UtilityCommand;
use keyring::{Entry, Error};
use log::info;

#[derive(Clone)]
pub struct Neuron {
    pub id: u64,
    pub auth: Auth,
}

impl Neuron {
    pub fn as_arg_vec(&self) -> Vec<String> {
        vec!["--proposer".to_string(), self.id.to_string()]
    }

    // FIXME: make this auth lazy
    pub async fn new(
        network: &ic_management_types::Network,
        require_authentication: bool,
        neuron_id: Option<u64>,
        private_key_pem: Option<String>,
        hsm_slot: Option<u64>,
        hsm_pin: Option<String>,
        hsm_key_id: Option<String>,
    ) -> anyhow::Result<Self> {
        match require_authentication {
            // Auth required, try to find valid neuron id using HSM or with the private key
            true => {
                // If private key is provided, use it without checking
                if let Some(path) = private_key_pem {
                    Ok(Self {
                        id: neuron_id.context("Neuron ID is required when using a private key")?,
                        auth: Auth::Keyfile { path },
                    })
                // If HSM slot, pin and key id are provided, use them without checking
                } else if let (Some(slot), Some(pin), Some(key_id)) = (hsm_slot, hsm_pin, hsm_key_id) {
                    Ok(Self {
                        id: neuron_id.context("Neuron ID is required when using HSM")?,
                        auth: Auth::Hsm { pin, slot, key_id },
                    })
                } else {
                    // Fully automatic detection of the neuron id using HSM
                    let auth = match detect_hsm_auth()? {
                        Some(auth) => auth,
                        None => return Err(anyhow::anyhow!("No HSM detected")),
                    };
                    match auto_detect_neuron(&network.get_nns_urls(), auth).await {
                        Ok(Some(n)) => Ok(n),
                        Ok(None) => anyhow::bail!("No HSM detected. Please provide HSM slot, pin, and key id."),
                        Err(e) => anyhow::bail!("Error while detectin neuron: {}", e),
                    }
                }
            }
            // Auth not required, don't attempt to talk to HSM and the NNS, but accept values provided by the user.
            false => {
                if let Some(path) = private_key_pem {
                    Ok(Self {
                        id: neuron_id.unwrap_or_default(),
                        auth: Auth::Keyfile { path },
                    })
                // If HSM slot, pin and key id are provided, use them without checking
                } else if let (Some(slot), Some(pin), Some(key_id)) = (hsm_slot, hsm_pin, hsm_key_id) {
                    Ok(Self {
                        id: neuron_id.unwrap_or_default(),
                        auth: Auth::Hsm { pin, slot, key_id },
                    })
                } else {
                    Ok(Self {
                        id: neuron_id.unwrap_or_default(),
                        auth: Auth::Keyfile {
                            path: "/fake/path/to/private_key.pem".to_string(),
                        },
                    })
                }
            }
        }
    }
}

#[derive(Clone)]
pub enum Auth {
    Hsm { pin: String, slot: u64, key_id: String },
    Keyfile { path: String },
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
            Auth::Keyfile { path } => vec!["--secret-key-pem".to_string(), path.clone()],
        }
    }

    pub fn from_cli_args(
        private_key_pem: Option<String>,
        hsm_slot: Option<u64>,
        hsm_pin: Option<String>,
        hsm_key_id: Option<String>,
    ) -> anyhow::Result<Self> {
        match (private_key_pem, hsm_slot, hsm_pin, hsm_key_id) {
            (Some(path), _, _, _) => Ok(Auth::Keyfile { path }),
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
            let pin_entry = Entry::new("release-cli", &key_id)?;
            let pin = match pin_entry.get_password() {
                Err(Error::NoEntry) => Password::new().with_prompt("Please enter the HSM PIN: ").interact()?,
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

// FIXME: This function should use either the HSM or the private key, instead of assuming the HSM
pub async fn auto_detect_neuron(nns_urls: &Vec<url::Url>, auth: Auth) -> anyhow::Result<Option<Neuron>> {
    if let Auth::Hsm { pin, slot, key_id } = auth {
        let auth = Auth::Hsm {
            pin: pin.clone(),
            slot,
            key_id: key_id.clone(),
        };
        let sender = Sender::from_external_hsm(
            UtilityCommand::read_public_key(Some(&slot.to_string()), Some(&key_id)).execute()?,
            std::sync::Arc::new(move |input| {
                Ok(
                    UtilityCommand::sign_message(input.to_vec(), Some(&slot.to_string()), Some(&pin), Some(&key_id))
                        .execute()?,
                )
            }),
        );
        let agent = Agent::new(nns_urls[0].clone(), sender);
        let neuron_id = if let Some(response) = agent
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
                0 => return Err(anyhow::anyhow!("HSM doesn't control any neurons")),
                1 => neuron_ids[0],
                _ => Select::with_theme(&ColorfulTheme::default())
                    .items(&neuron_ids)
                    .default(0)
                    .interact_on_opt(&Term::stderr())?
                    .map(|i| neuron_ids[i])
                    .ok_or_else(|| anyhow::anyhow!("No neuron selected"))?,
            }
        } else {
            return Err(anyhow::anyhow!("Empty response when listing controlled neurons"));
        };

        Ok(Some(Neuron { id: neuron_id, auth }))
    } else {
        Ok(None)
    }
}
