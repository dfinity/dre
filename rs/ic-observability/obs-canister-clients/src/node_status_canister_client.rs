use std::time::Duration;

use candid::{CandidType, Decode, Encode};
use ic_agent::agent::http_transport::ReqwestTransport;
use ic_agent::agent::CallResponse;
use ic_agent::{export::Principal, identity::AnonymousIdentity, Agent};
use rand::seq::SliceRandom;
use serde::Deserialize;
use url::Url;

pub struct NodeStatusCanister {
    canister_id: Principal,
    agent: Vec<Agent>,
}

#[derive(Debug)]
pub enum NodeStatusCanisterError {
    Encoding(String),
    Decoding(String),
    Unknown(String),
}

#[derive(CandidType, Default)]
struct Argument {}

#[derive(CandidType, Deserialize, Debug, PartialEq, Clone)]
pub struct NodeStatus {
    pub node_id: Principal,
    pub subnet_id: Option<Principal>,
    pub status: bool,
}

impl Default for NodeStatus {
    fn default() -> Self {
        NodeStatus {
            node_id: Principal::anonymous(),
            subnet_id: None,
            status: false,
        }
    }
}

impl NodeStatusCanister {
    pub fn new(url: Vec<Url>, canister_id: String) -> Self {
        assert!(!url.is_empty(), "empty list of URLs passed to NodeStatusCanister::new()");

        NodeStatusCanister {
            canister_id: Principal::from_text(canister_id).unwrap(),
            agent: url
                .iter()
                .map(|url| {
                    let client = reqwest::Client::builder()
                        .use_rustls_tls()
                        .timeout(Duration::from_secs(30))
                        .build()
                        .expect("Could not create HTTP client.");
                    Agent::builder()
                        .with_transport(ReqwestTransport::create_with_client(url.as_str(), client).expect("Failed to create transport"))
                        .with_identity(AnonymousIdentity)
                        .with_verify_query_signatures(false)
                        .build()
                        .expect("Failed to build agent")
                })
                .collect(),
        }
    }

    async fn choose_random_agent(&self) -> &Agent {
        let agent = self
            .agent
            .choose(&mut rand::thread_rng())
            .expect("can't fail, ::new asserts list is non-empty");

        if agent.read_root_key().iter().any(|k| *k != 0) {
            agent.fetch_root_key().await.unwrap();
        }

        agent
    }

    pub async fn get_node_status(&self, format_for_frontend: bool) -> Result<Vec<NodeStatus>, NodeStatusCanisterError> {
        match self
            .choose_random_agent()
            .await
            .query(&self.canister_id, "get_node_status")
            .with_effective_canister_id(self.canister_id)
            .with_arg(
                Encode! { &format_for_frontend }
                    .map_err(|err| NodeStatusCanisterError::Encoding(format!("Error encoding argument for get_node_status: {}", err)))?,
            )
            .call()
            .await
        {
            Ok(result) => match Decode!(result.as_slice(), Vec<NodeStatus>) {
                Ok(response) => Ok(response),
                Err(e) => Err(NodeStatusCanisterError::Decoding(format!(
                    "Error decoding response for get_node_status: {}",
                    e
                ))),
            },
            Err(err) => Err(NodeStatusCanisterError::Unknown(format!("Error on get_node_status: {}", err))),
        }
    }

    pub async fn update_node_statuses(&self, statuses: Vec<NodeStatus>) -> Result<bool, NodeStatusCanisterError> {
        let response = match self
            .choose_random_agent()
            .await
            .update(&self.canister_id, "update_node_status")
            .with_effective_canister_id(self.canister_id)
            .with_arg(
                Encode! { &statuses }
                    .map_err(|err| NodeStatusCanisterError::Encoding(format!("Error encoding argument for update_node_status: {}", err)))?,
            )
            .call()
            .await
        {
            Ok(CallResponse::Response(response)) => response,
            Ok(CallResponse::Poll(request_id)) => self
                .choose_random_agent()
                .await
                .wait(&request_id, self.canister_id)
                .await
                .map_err(|err| NodeStatusCanisterError::Unknown(format!("Error on getting response for update_node_status: {}", err)))?,
            Err(err) => return Err(NodeStatusCanisterError::Unknown(format!("Error on update_node_status request: {}", err))),
        };

        match Decode!(response.as_slice(), bool) {
            Ok(response) => Ok(response),
            Err(e) => Err(NodeStatusCanisterError::Decoding(format!(
                "Error decoding response for update_node_status: {}",
                e
            ))),
        }
    }
}
