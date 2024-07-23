use candid::CandidType;
use ic_agent::agent::http_transport::ReqwestTransport;
use ic_agent::identity::AnonymousIdentity;
use ic_agent::identity::BasicIdentity;
use ic_agent::identity::Secp256k1Identity;
use ic_agent::Agent;
use ic_agent::Identity;
use ic_base_types::CanisterId;
use ic_canister_client::Agent as CanisterClientAgent;
use ic_canister_client::Sender;
use ic_canister_client_sender::SigKeys;
use ic_sys::utility_command::UtilityCommand;
use parallel_hardware_identity::ParallelHardwareIdentity;
use serde::Deserialize;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Mutex;
use std::time::Duration;
use url::Url;

pub mod governance;
pub mod management;
pub mod node_metrics;
pub mod parallel_hardware_identity;
pub mod registry;
pub mod sns_wasm;

pub struct CanisterClient {
    pub agent: CanisterClientAgent,
}

impl CanisterClient {
    pub fn from_hsm(pin: String, slot: u64, key_id: String, nns_url: &Url) -> anyhow::Result<Self> {
        let sender = Sender::from_external_hsm(
            UtilityCommand::read_public_key(Some(&slot.to_string()), Some(&key_id)).execute()?,
            std::sync::Arc::new(move |input| {
                Ok(UtilityCommand::sign_message(input.to_vec(), Some(&slot.to_string()), Some(&pin), Some(&key_id)).execute()?)
            }),
        );

        Ok(Self {
            agent: CanisterClientAgent::new(nns_url.clone(), sender),
        })
    }

    pub fn from_key_file(file: PathBuf, nns_url: &Url) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(file).expect("Could not read key file");
        let sig_keys = SigKeys::from_pem(&contents).expect("Failed to parse pem file");
        let sender = Sender::SigKeys(sig_keys);

        Ok(Self {
            agent: CanisterClientAgent::new(nns_url.clone(), sender),
        })
    }

    pub fn from_anonymous(nns_url: &Url) -> anyhow::Result<Self> {
        Ok(Self {
            agent: CanisterClientAgent::new(nns_url.clone(), Sender::Anonymous),
        })
    }
}

pub struct IcAgentCanisterClient {
    pub agent: Agent,
}

impl IcAgentCanisterClient {
    pub fn from_key_file(path: PathBuf, url: Url) -> anyhow::Result<Self> {
        let identity: Box<dyn Identity> = if let Ok(identity) = BasicIdentity::from_pem_file(&path) {
            Box::new(identity)
        } else {
            let identity = Secp256k1Identity::from_pem_file(&path).map_err(|e| anyhow::anyhow!("Couldn't load identity: {:?}", e))?;
            Box::new(identity)
        };
        Self::build_agent(url, identity)
    }

    pub fn from_hsm(pin: String, slot: u64, key_id: String, url: Url, lock: Option<Mutex<()>>) -> anyhow::Result<Self> {
        let pin_fn = || Ok(pin);
        let identity = ParallelHardwareIdentity::new(pkcs11_lib_path()?, slot as usize, &key_id, pin_fn, lock)?;
        Self::build_agent(url, Box::new(identity))
    }

    pub fn from_anonymous(url: Url) -> anyhow::Result<Self> {
        Self::build_agent(url, Box::new(AnonymousIdentity))
    }

    fn build_agent(url: Url, identity: Box<dyn Identity>) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Could not create HTTP client.");
        let agent = Agent::builder()
            .with_identity(identity)
            .with_transport(ReqwestTransport::create_with_client(url, client)?)
            .with_verify_query_signatures(false)
            .build()?;
        Ok(Self { agent })
    }
}

#[derive(Clone, CandidType, Deserialize, Debug)]
pub struct CallIn<TCycles = u128> {
    canister: CanisterId,
    method_name: String,
    args: Vec<u8>,
    cycles: TCycles,
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
