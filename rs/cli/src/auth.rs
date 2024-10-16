use std::path::PathBuf;
use std::str::FromStr;

use anyhow::anyhow;
use clio::{ClioPath, InputPath};
use cryptoki::object::AttributeInfo;
use cryptoki::session::Session;
use cryptoki::{
    context::{CInitializeArgs, Pkcs11},
    object::{Attribute, AttributeType},
    session::UserType,
    slot::{Slot, TokenInfo},
};
use dialoguer::{console::Term, theme::ColorfulTheme, Password, Select};
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_canisters::IcAgentCanisterClient;
use ic_icrc1_test_utils::KeyPairGenerator;
use ic_management_types::Network;
use keyring::{Entry, Error};
use log::{debug, info, warn};
use secrecy::SecretString;
use std::sync::Mutex;

use crate::commands::{AuthOpts, AuthRequirement, HsmOpts, HsmParams};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Neuron {
    pub auth: Auth,
    pub neuron_id: u64,
    pub include_proposer: bool,
}

pub const STAGING_NEURON_ID: u64 = 49;
pub const STAGING_KEY_PATH_FROM_HOME: &str = ".config/dfx/identity/bootstrap-super-leader/identity.pem";

// As per fn str_to_key_id(s: &str) in ic-canisters/.../parallel_hardware_identity.rs,
// the representation of key ID that the canister client wants is a sequence of
// pairs of hex digits, case-insensitive.  The key ID as stored in the HSM is
// a Vec<u8>.  We only store the little-endianest of the digits from that Vec<> in
// our key_id variable.  The following function produces what the ic-canisters
// code wants.
pub fn hsm_key_id_to_string(s: u8) -> String {
    format!("{:02x?}", s)
}

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
                            private_key_pem: Some(InputPath::new(ClioPath::new(staging_known_path).unwrap()).unwrap()),
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

        Self::from_opts_and_req_inner(auth_opts, requirement, network, neuron_id, offline).await
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

    async fn from_opts_and_req_inner(
        auth_opts: AuthOpts,
        requirement: AuthRequirement,
        network: &Network,
        neuron_id: Option<u64>,
        offline: bool,
    ) -> anyhow::Result<Self> {
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
                let auth = Auth::from_auth_opts(auth_opts).await?;
                Self {
                    neuron_id: match (neuron_id, offline) {
                        (Some(n), _) => n,
                        // This is just a placeholder since
                        // the tool is instructed to run in
                        // offline mode.
                        (None, true) => {
                            warn!("Required full neuron but offline mode instructed! Will not attempt to auto-detect neuron id");
                            0
                        }
                        (None, false) => auth.auto_detect_neuron_id(network.nns_urls.clone()).await?,
                    },
                    auth,
                    include_proposer: true,
                }
            }),
        }
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Auth {
    Hsm { pin: String, slot: u64, key_id: u8 },
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
                key_id.to_string(),
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

    pub fn create_canister_client(&self, nns_urls: Vec<url::Url>, lock: Option<Mutex<()>>) -> anyhow::Result<IcAgentCanisterClient> {
        // FIXME: why do we even take multiple URLs if only the first one is ever used?
        let url = nns_urls.first().ok_or(anyhow::anyhow!("No NNS URLs provided"))?.to_owned();
        match self {
            Auth::Hsm { pin, slot, key_id } => IcAgentCanisterClient::from_hsm(pin.clone(), *slot, hsm_key_id_to_string(*key_id), url, lock),
            Auth::Keyfile { path } => IcAgentCanisterClient::from_key_file(path.clone(), url),
            Auth::Anonymous => IcAgentCanisterClient::from_anonymous(url),
        }
    }

    async fn auto_detect_neuron_id(&self, nns_urls: Vec<url::Url>) -> anyhow::Result<u64> {
        let selfclone = self.clone();
        let nnsurlsclone = nns_urls.clone();
        let client = tokio::task::spawn_blocking(move || selfclone.create_canister_client(nnsurlsclone, None)).await??;
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
    fn detect_hsm_auth(maybe_pin: Option<String>, maybe_slot: Option<u64>, maybe_key_id: Option<u8>) -> anyhow::Result<Self> {
        if maybe_slot.is_none() && maybe_key_id.is_none() {
            debug!("Scanning hardware security module devices");
        }
        if let Some(slot) = &maybe_slot {
            debug!("Probing hardware security module in slot {}", slot);
        }
        if let Some(key_id) = &maybe_key_id {
            debug!("Limiting key scan to keys with ID {}", key_id);
        }

        let ctx = Pkcs11::new(Self::pkcs11_lib_path()?)?;
        ctx.initialize(CInitializeArgs::OsThreads)?;
        for slot in ctx.get_slots_with_token()? {
            let info = ctx.get_slot_info(slot)?;
            let token_info = ctx.get_token_info(slot)?;
            if info.slot_description().starts_with("Nitrokey Nitrokey HSM") && maybe_slot.is_none() || (maybe_slot.unwrap() == slot.id()) {
                let session = ctx.open_ro_session(slot)?;
                let key_id = match Auth::find_key_id_in_slot_session(&session, maybe_key_id)? {
                    Some((key_id, label)) => {
                        debug!(
                            "Found key with ID {} ({}) in slot {}",
                            key_id,
                            match label {
                                Some(label) => format!("labeled {}", label),
                                None => "without label".to_string(),
                            },
                            slot.id()
                        );
                        key_id
                    }
                    None => {
                        if maybe_slot.is_some() && maybe_key_id.is_some() {
                            // We have been asked to be very specific.  Fail fast,
                            // instead of falling back to Auth::Anonymous.
                            return Err(anyhow!(
                                "Could not find a key ID {} within hardware security module in slot {}",
                                maybe_key_id.unwrap(),
                                slot.id()
                            ));
                        } else {
                            // Let's try the next slot just in case.
                            continue;
                        }
                    }
                };
                let memo_key: String = format!("hsm-{}-{}", info.slot_description(), info.manufacturer_id());
                let pin = Auth::get_or_prompt_pin_checked_for_slot(&session, maybe_pin, slot, token_info, &memo_key)?;
                let detected = Auth::Hsm {
                    pin,
                    slot: slot.id(),
                    key_id,
                };
                info!("Using key ID {} of hardware security module in slot {}", key_id, slot);
                return Ok(detected);
            }
        }
        Err(anyhow!(
            "No hardware security module detected{}{}",
            match maybe_slot {
                None => "".to_string(),
                Some(slot) => format!(" in slot {}", slot),
            },
            match maybe_key_id {
                None => "".to_string(),
                Some(key_id) => format!(" that contains a key ID {}", key_id),
            }
        ))
    }

    /// Finds the key ID in a slot.  If a key ID is specified,
    /// then the search is limited to that key ID.  If not, then
    /// the first key that has an ID and is for a token is returned.
    /// If a key is found, this function returns Some, with a tuple of
    /// the found key ID, and possibly the label assigned to said key ID
    /// (None if no / invalid label).
    fn find_key_id_in_slot_session(session: &Session, key_id: Option<u8>) -> anyhow::Result<Option<(u8, Option<String>)>> {
        let token_types = vec![AttributeType::Token, AttributeType::Id];
        let label_types = vec![AttributeType::Label];
        let objects = session.find_objects(&[])?;
        for hnd in objects.iter() {
            if let [AttributeInfo::Available(_), AttributeInfo::Available(_)] = session.get_attribute_info(*hnd, &token_types)?[0..token_types.len()]
            {
                // Object may be a token and has an ID.
                if let [Attribute::Token(true), Attribute::Id(token_id)] = &session.get_attributes(*hnd, &token_types)?[0..token_types.len()] {
                    // Object is a token, and we have extracted the ID.
                    if !token_id.is_empty() && (key_id.is_none() || token_id[0] == key_id.unwrap()) {
                        let found_key_id = token_id[0];
                        let mut label: Option<String> = None;
                        if let [AttributeInfo::Available(_)] = &session.get_attribute_info(*hnd, &label_types)?[0..label_types.len()] {
                            // Object has a label.
                            if let [Attribute::Label(token_label)] = &session.get_attributes(*hnd, &label_types)?[0..label_types.len()] {
                                // We have extracted the label; we make a copy of it.
                                label = match String::from_utf8(token_label.clone()) {
                                    Ok(label) => Some(label),
                                    Err(_) => None,
                                }
                            }
                        }
                        return Ok(Some((found_key_id, label)));
                    }
                }
            }
        }
        Ok(None)
    }

    fn get_or_prompt_pin_checked_for_slot(
        session: &Session,
        maybe_pin: Option<String>,
        slot: Slot,
        token_info: TokenInfo,
        memo_key: &str,
    ) -> anyhow::Result<String> {
        if token_info.user_pin_locked() {
            return Err(anyhow!("The PIN for the token stored in slot {} is locked", slot.id()));
        }
        if token_info.user_pin_final_try() {
            warn!(
                "The PIN for the token stored in slot {} is at its last try, and if this operation fails, the token will be locked",
                slot.id()
            );
        }
        let ret = Ok(match maybe_pin {
            Some(pin) => {
                let sekrit = SecretString::from_str(pin.as_str()).unwrap();
                session.login(UserType::User, Some(&sekrit))?;
                pin
            }
            None => {
                let pin_entry = Entry::new("dre-tool-hsm-pin", memo_key)?;
                let tentative_pin = match pin_entry.get_password() {
                    // TODO: Remove the old keyring entry search ("release-cli") after August 1st, 2024
                    Err(Error::NoEntry) => match Entry::new("release-cli", memo_key) {
                        Err(Error::NoEntry) => Password::new()
                            .with_prompt("Please enter the hardware security module PIN: ")
                            .interact()?,
                        Ok(pin_entry) => pin_entry.get_password()?,
                        Err(e) => return Err(anyhow::anyhow!("Problem getting PIN from keyring: {}", e)),
                    },
                    Ok(pin) => pin,
                    Err(e) => return Err(anyhow::anyhow!("Problem getting from keyring: {}", e)),
                };
                let sekrit = SecretString::from_str(tentative_pin.as_str()).unwrap();
                match session.login(UserType::User, Some(&sekrit)) {
                    Ok(_) => {
                        pin_entry.set_password(&tentative_pin)?;
                        tentative_pin
                    }
                    Err(e) => {
                        pin_entry.delete_credential()?;
                        return Err(anyhow!("Hardware security module PIN incorrect ({})", e));
                    }
                }
            }
        });
        info!("Hardware security module PIN correct");
        ret
    }

    /// Create an Auth that automatically detects an HSM.  Falls back to
    /// anonymous authentication if no HSM is detected.  Prompts the user
    /// for a PIN if no PIN is specified and the HSM needs to be unlocked.
    /// Caller can optionally limit search to a specific slot or key ID.
    pub async fn auto(hsm_pin: Option<String>, hsm_slot: Option<u64>, hsm_key_id: Option<u8>) -> anyhow::Result<Self> {
        tokio::task::spawn_blocking(move || Self::detect_hsm_auth(hsm_pin, hsm_slot, hsm_key_id)).await?
    }

    pub async fn pem(private_key_pem: PathBuf) -> anyhow::Result<Self> {
        // Check path exists.  This blocks.
        let t = tokio::task::spawn_blocking(move || {
            let inp = InputPath::new(&private_key_pem);
            match inp {
                Ok(inp) => Ok(inp.path().to_path_buf()),
                Err(e) => Err(e),
            }
        })
        .await?;
        Ok(Self::Keyfile { path: t? })
    }

    pub(crate) async fn from_auth_opts(auth_opts: AuthOpts) -> Result<Self, anyhow::Error> {
        match &auth_opts {
            // Private key case.
            AuthOpts {
                private_key_pem: Some(private_key_pem),
                hsm_opts: _,
            } => {
                info!("Using requested private key file {}", private_key_pem.path());
                Auth::pem(private_key_pem.path().to_path_buf()).await
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
            } => Auth::auto(pin.clone(), *hsm_slot, *hsm_key_id).await,
        }
    }
}
