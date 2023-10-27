use candid::{Decode, Encode};
use ic_agent::Agent;
use ic_canister_client::Agent as CanisterClientAgent;
use ic_canister_client::Sender;
use ic_canister_client_sender::SigKeys;
use ic_nns_common::pb::v1::NeuronId;
use ic_nns_common::pb::v1::ProposalId;
use ic_nns_constants::GOVERNANCE_CANISTER_ID;
use ic_nns_governance::pb::v1::manage_neuron::RegisterVote;
use ic_nns_governance::pb::v1::manage_neuron_response::RegisterVoteResponse;
use ic_nns_governance::pb::v1::ManageNeuron;
use ic_nns_governance::pb::v1::ManageNeuronResponse;
use ic_nns_governance::pb::v1::ProposalInfo;
use ic_sys::utility_command::UtilityCommand;
use serde::{self, Serialize};
use std::path::PathBuf;
use std::str::FromStr;
use url::Url;

#[derive(Clone, Serialize)]
pub struct GovernanceCanisterVersion {
    pub stringified_hash: String,
}

pub async fn governance_canister_version(nns_url: Url) -> Result<GovernanceCanisterVersion, anyhow::Error> {
    let canister_agent = Agent::builder()
        .with_transport(ic_agent::agent::http_transport::ReqwestHttpReplicaV2Transport::create(
            nns_url,
        )?)
        .build()?;

    canister_agent.fetch_root_key().await?;

    let governance_canister_build = std::str::from_utf8(
        &canister_agent
            .read_state_canister_metadata(
                candid::Principal::from_str(&GOVERNANCE_CANISTER_ID.to_string())
                    .expect("failed to convert governance canister principal to candid type"),
                "git_commit_id",
            )
            .await?,
    )?
    .trim()
    .to_string();

    Ok(GovernanceCanisterVersion {
        stringified_hash: governance_canister_build,
    })
}

pub struct GovernanceCanisterWrapper {
    agent: CanisterClientAgent,
}

impl GovernanceCanisterWrapper {
    pub fn from_hsm(pin: String, slot: u64, key_id: String, nns_url: &Url) -> anyhow::Result<Self> {
        let sender = Sender::from_external_hsm(
            UtilityCommand::read_public_key(Some(&slot.to_string()), Some(&key_id)).execute()?,
            std::sync::Arc::new(move |input| {
                Ok(
                    UtilityCommand::sign_message(input.to_vec(), Some(&slot.to_string()), Some(&pin), Some(&key_id))
                        .execute()?,
                )
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

    pub async fn get_pending_proposals(&self) -> anyhow::Result<Vec<ProposalInfo>> {
        let vec: Vec<u8> = vec![];
        match self
            .agent
            .execute_query(&GOVERNANCE_CANISTER_ID, "get_pending_proposals", Encode! { &vec }?)
            .await
        {
            Ok(Some(response)) => match Decode!(response.as_slice(), Vec<ProposalInfo>) {
                Ok(response) => Ok(response),
                Err(err) => Err(anyhow::anyhow!("Error decoding response: {}", err)),
            },
            Ok(None) => Ok(vec![]),
            Err(err) => Err(anyhow::anyhow!("Error executing query: {}", err)),
        }
    }

    pub async fn register_vote(&self, neuron_id: u64, proposal_id: u64) -> anyhow::Result<RegisterVoteResponse> {
        let response = self
            .manage_neuron(&ManageNeuron {
                id: Some(NeuronId { id: neuron_id }),
                neuron_id_or_subaccount: None,
                command: Some(ic_nns_governance::pb::v1::manage_neuron::Command::RegisterVote(
                    RegisterVote {
                        proposal: Some(ProposalId { id: proposal_id }),
                        vote: ic_nns_governance::pb::v1::Vote::Yes.into(),
                    },
                )),
            })
            .await?;
        match response.command {
            Some(ic_nns_governance::pb::v1::manage_neuron_response::Command::RegisterVote(response)) => Ok(response),
            _ => Err(anyhow::anyhow!("Error registering vote")),
        }
    }

    async fn manage_neuron(&self, manage_neuron: &ManageNeuron) -> anyhow::Result<ManageNeuronResponse> {
        let vec: Vec<u8> = vec![];

        match self
            .agent
            .execute_update(
                &GOVERNANCE_CANISTER_ID,
                &GOVERNANCE_CANISTER_ID,
                "manage_neuron",
                Encode! { manage_neuron }?,
                Encode! { &vec }?,
            )
            .await
        {
            Ok(Some(response)) => match Decode!(response.as_slice(), ManageNeuronResponse) {
                Ok(response) => Ok(response),
                Err(err) => Err(anyhow::anyhow!("Error decoding response: {}", err)),
            },
            Ok(None) => Ok(ManageNeuronResponse::default()),
            Err(err) => Err(anyhow::anyhow!("Error executing update: {}", err)),
        }
    }
}
