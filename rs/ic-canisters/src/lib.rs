use candid::CandidType;
use candid::Decode;
use candid::Principal;
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
use serde::Serialize;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use url::Url;

pub mod cycles_minting;
pub mod governance;
pub mod ledger;
pub mod management;
pub mod node_metrics;
pub mod node_rewards;
pub mod parallel_hardware_identity;
pub mod registry;
pub mod sns_wasm;

#[derive(Clone)]
pub struct IcAgentCanisterClient {
    pub agent: Agent,
    pub nns_url: Url,
}

#[derive(Clone, Serialize)]
pub struct CanisterVersion {
    pub stringified_hash: String,
}

pub(crate) async fn canister_version(url: Url, canister_id: Principal) -> anyhow::Result<CanisterVersion> {
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Could not create HTTP client.");
    let canister_agent = Agent::builder()
        .with_http_client(client)
        .with_url(url)
        .with_verify_query_signatures(false)
        .build()?;

    canister_agent.fetch_root_key().await?;

    let governance_canister_build = std::str::from_utf8(&canister_agent.read_state_canister_metadata(canister_id, "git_commit_id").await?)?
        .trim()
        .to_string();

    Ok(CanisterVersion {
        stringified_hash: governance_canister_build,
    })
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

    pub fn from_hsm(identity: ParallelHardwareIdentity, url: Url) -> anyhow::Result<Self> {
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
            .with_http_client(client)
            .with_url(url.clone())
            .with_verify_query_signatures(false)
            .build()?;
        Ok(Self { agent, nns_url: url })
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
