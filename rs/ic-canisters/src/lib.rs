use candid::CandidType;
use candid::Decode;
use candid::Principal;
use ic_agent::agent::http_transport::ReqwestTransport;
use ic_agent::identity::AnonymousIdentity;
use ic_agent::identity::BasicIdentity;
use ic_agent::identity::Secp256k1Identity;
use ic_agent::Agent;
use ic_agent::Identity;
use ic_base_types::CanisterId;
use ic_base_types::PrincipalId;
use ic_transport_types::SubnetMetrics;
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

    pub async fn read_state_subnet_metrics(&self, subnet_id: &PrincipalId) -> anyhow::Result<SubnetMetrics> {
        self.agent
            .read_state_subnet_metrics(candid::Principal::from_str(subnet_id.to_string().as_str())?)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    async fn query<T>(&self, canister_id: &Principal, method_name: &str, args: Vec<u8>) -> anyhow::Result<T>
    where
        T: candid::CandidType + for<'a> candid::Deserialize<'a>,
    {
        self.agent
            .query(canister_id, method_name)
            .with_arg(args)
            .call()
            .await
            .map_err(anyhow::Error::from)
            .map(|r| Decode!(r.as_slice(), T))?
            .map_err(|e| anyhow::anyhow!("Error while decoding into {}: {:?}", std::any::type_name::<T>(), e))
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
