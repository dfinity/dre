use candid::Decode;
use ic_agent::Agent;
use ic_nns_common::pb::v1::NeuronId;
use ic_nns_common::pb::v1::ProposalId;
use ic_nns_constants::GOVERNANCE_CANISTER_ID;
use ic_nns_governance::pb::v1::manage_neuron::RegisterVote;
use ic_nns_governance::pb::v1::ListProposalInfo;
use ic_nns_governance::pb::v1::ListProposalInfoResponse;
use ic_nns_governance::pb::v1::ManageNeuron;
use ic_nns_governance::pb::v1::ManageNeuronResponse;
use ic_nns_governance::pb::v1::ProposalInfo;
use log::warn;
use serde::{self, Serialize};
use std::str::FromStr;
use url::Url;

use crate::CanisterClient;
const MAX_RETRIES: usize = 5;

#[derive(Clone, Serialize)]
pub struct GovernanceCanisterVersion {
    pub stringified_hash: String,
}

pub async fn governance_canister_version(nns_urls: &[Url]) -> Result<GovernanceCanisterVersion, anyhow::Error> {
    let canister_agent = Agent::builder()
        .with_transport(ic_agent::agent::http_transport::reqwest_transport::ReqwestHttpReplicaV2Transport::create(
            nns_urls[0].clone(),
        )?)
        .with_verify_query_signatures(false)
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
    client: CanisterClient,
}

impl From<CanisterClient> for GovernanceCanisterWrapper {
    fn from(value: CanisterClient) -> Self {
        Self { client: value }
    }
}

impl GovernanceCanisterWrapper {
    pub async fn get_pending_proposals(&self) -> anyhow::Result<Vec<ProposalInfo>> {
        let mut retries = 0;
        backoff::future::retry(backoff::ExponentialBackoff::default(), || async move {
            retries += 1;
            if retries >= MAX_RETRIES {
                return Err(backoff::Error::Permanent(anyhow::anyhow!("Max retries exceeded")));
            }
            let empty_args = candid::encode_one(()).map_err(|err| backoff::Error::Permanent(anyhow::format_err!(err)))?;
            match self
                .client
                .agent
                .execute_query(&GOVERNANCE_CANISTER_ID, "get_pending_proposals", empty_args)
                .await
            {
                Ok(Some(response)) => match Decode!(response.as_slice(), Vec<ProposalInfo>) {
                    Ok(response) => Ok(response),
                    Err(err) => Err(backoff::Error::Permanent(anyhow::anyhow!("Error decoding response: {}", err))),
                },
                Ok(None) => Ok(vec![]),
                Err(err) => {
                    warn!("Error executing query, retrying: {}", err);
                    Err(backoff::Error::Transient {
                        err: anyhow::anyhow!("Error executing query: {}", err),
                        retry_after: None,
                    })
                }
            }
        })
        .await
    }

    pub async fn get_proposal(&self, proposal_id: u64) -> anyhow::Result<ProposalInfo> {
        let mut retries = 0;
        backoff::future::retry(backoff::ExponentialBackoff::default(), || async move {
            retries += 1;
            if retries >= MAX_RETRIES {
                return Err(backoff::Error::Permanent(anyhow::anyhow!("Max retries exceeded")));
            }
            let args = candid::encode_one(proposal_id).map_err(|err| backoff::Error::Permanent(anyhow::format_err!(err)))?;
            match self.client.agent.execute_query(&GOVERNANCE_CANISTER_ID, "get_proposal_info", args).await {
                Ok(Some(response)) => match Decode!(response.as_slice(), Option<ProposalInfo>) {
                    Ok(response) => match response {
                        Some(proposal) => Ok(proposal),
                        None => Err(backoff::Error::Permanent(anyhow::anyhow!("Proposal with id {} not found", proposal_id))),
                    },
                    Err(err) => Err(backoff::Error::Permanent(anyhow::anyhow!("Error decoding response: {}", err))),
                },
                Ok(None) => Err(backoff::Error::Permanent(anyhow::anyhow!("Got an empty reponse"))),
                Err(err) => {
                    warn!("Error executing query, retrying: {}", err);
                    Err(backoff::Error::Transient {
                        err: anyhow::anyhow!("Error executing query: {}", err),
                        retry_after: None,
                    })
                }
            }
        })
        .await
    }

    pub async fn register_vote(&self, neuron_id: u64, proposal_id: u64) -> anyhow::Result<String> {
        let mut retries = 0;
        let response = backoff::future::retry(backoff::ExponentialBackoff::default(), || async move {
            retries += 1;
            if retries >= MAX_RETRIES {
                return Err(backoff::Error::Permanent(anyhow::anyhow!("Max retries exceeded")));
            }
            self.manage_neuron(&ManageNeuron {
                id: Some(NeuronId { id: neuron_id }),
                neuron_id_or_subaccount: None,
                command: Some(ic_nns_governance::pb::v1::manage_neuron::Command::RegisterVote(RegisterVote {
                    proposal: Some(ProposalId { id: proposal_id }),
                    vote: ic_nns_governance::pb::v1::Vote::Yes.into(),
                })),
            })
            .await
            .map_err(|err| backoff::Error::Transient { err, retry_after: None })
        })
        .await?;

        match response.command {
            Some(ic_nns_governance::pb::v1::manage_neuron_response::Command::RegisterVote(response)) => {
                Ok(format!("Successfully voted on proposal {} {:?}", proposal_id, response))
            }
            Some(ic_nns_governance::pb::v1::manage_neuron_response::Command::Error(err))
                if err
                    == ic_nns_governance::pb::v1::GovernanceError {
                        error_type: ic_nns_governance::pb::v1::governance_error::ErrorTypeDesc::PreconditionFailed as i32,
                        error_message: "Neuron already voted on proposal.".to_string(),
                    } =>
            {
                Ok(format!("Neuron already voted on proposal {}, cannot vote again.", proposal_id))
            }
            _err => Err(anyhow::anyhow!("Error registering vote: {:?}", _err)),
        }
    }

    async fn manage_neuron(&self, manage_neuron: &ManageNeuron) -> anyhow::Result<ManageNeuronResponse> {
        match self
            .client
            .agent
            .execute_update(
                &GOVERNANCE_CANISTER_ID,
                &GOVERNANCE_CANISTER_ID,
                "manage_neuron",
                candid::encode_one(manage_neuron)?,
                candid::encode_one(())?,
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

    pub async fn list_proposals(&self, contract: ListProposalInfo) -> anyhow::Result<Vec<ProposalInfo>> {
        let args = candid::encode_one(&contract)?;
        match self.client.agent.execute_query(&GOVERNANCE_CANISTER_ID, "list_proposals", args).await {
            Ok(Some(response)) => match Decode!(response.as_slice(), ListProposalInfoResponse) {
                Ok(response) => Ok(response.proposal_info),
                Err(e) => Err(anyhow::anyhow!("Error deserializing response: {:?}", e)),
            },
            Ok(None) => Ok(vec![]),
            Err(e) => Err(anyhow::anyhow!("Error executing query: {}", e)),
        }
    }
}
