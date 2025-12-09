use candid::Decode;
use ic_nns_common::pb::v1::NeuronId;
use ic_nns_common::pb::v1::ProposalId;
use ic_nns_constants::GOVERNANCE_CANISTER_ID;
use ic_nns_governance::pb::v1::GovernanceError;
use ic_nns_governance::pb::v1::ManageNeuron;
use ic_nns_governance::pb::v1::NodeProvider as PbNodeProvider;
use ic_nns_governance::pb::v1::Proposal;
use ic_nns_governance::pb::v1::manage_neuron::ClaimOrRefresh;
use ic_nns_governance::pb::v1::manage_neuron::Command;
use ic_nns_governance::pb::v1::manage_neuron::Command::ClaimOrRefresh as CoR;
use ic_nns_governance::pb::v1::manage_neuron::NeuronIdOrSubaccount;
use ic_nns_governance::pb::v1::manage_neuron::RegisterVote;
use ic_nns_governance::pb::v1::manage_neuron::claim_or_refresh::By;
use ic_nns_governance_api::DateRangeFilter;
use ic_nns_governance_api::ListNodeProviderRewardsRequest;
use ic_nns_governance_api::ListNodeProviderRewardsResponse;
use ic_nns_governance_api::ListNodeProvidersResponse;
use ic_nns_governance_api::ListProposalInfoRequest;
use ic_nns_governance_api::ManageNeuronResponse;
use ic_nns_governance_api::MonthlyNodeProviderRewards;
use ic_nns_governance_api::manage_neuron_response::Command as CommandResponse;
use ic_nns_governance_api::manage_neuron_response::MakeProposalResponse;
use ic_nns_governance_api::{ListNeurons, ListProposalInfoResponse, NeuronInfo, ProposalInfo};
use ic_nns_governance_api::{ListNeuronsResponse, Neuron};
use url::Url;

use crate::CanisterVersion;
use crate::IcAgentCanisterClient;
const MAX_RETRIES: usize = 5;

pub async fn governance_canister_version(nns_urls: &[Url]) -> Result<CanisterVersion, anyhow::Error> {
    super::canister_version(nns_urls[0].clone(), GOVERNANCE_CANISTER_ID.into()).await
}

pub struct GovernanceCanisterWrapper {
    client: IcAgentCanisterClient,
}

impl From<IcAgentCanisterClient> for GovernanceCanisterWrapper {
    fn from(value: IcAgentCanisterClient) -> Self {
        Self { client: value }
    }
}

impl<T> From<(T, IcAgentCanisterClient)> for GovernanceCanisterWrapper {
    fn from(value: (T, IcAgentCanisterClient)) -> Self {
        let (_, client) = value;
        Self { client }
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
            self.query(
                "get_pending_proposals",
                candid::encode_one(()).map_err(|err| backoff::Error::Permanent(anyhow::format_err!(err)))?,
            )
            .await
            .map_err(backoff::Error::permanent)
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
            self.query::<Option<ProposalInfo>>(
                "get_proposal_info",
                candid::encode_one(proposal_id).map_err(|err| backoff::Error::Permanent(anyhow::format_err!(err)))?,
            )
            .await
            .map_err(|e| backoff::Error::transient(anyhow::format_err!(e)))?
            .ok_or(backoff::Error::permanent(anyhow::anyhow!("Failed to find proposal {}", proposal_id)))
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
            None => Err(anyhow::anyhow!("No command in response")),
            Some(CommandResponse::RegisterVote(response)) => Ok(format!("Successfully voted on proposal {} {:?}", proposal_id, response)),
            Some(CommandResponse::Error(err))
                if err
                    == ic_nns_governance_api::GovernanceError {
                        error_type: ic_nns_governance::pb::v1::governance_error::ErrorTypeDesc::PreconditionFailed as i32,
                        error_message: "Neuron already voted on proposal.".to_string(),
                    } =>
            {
                Ok(format!("Neuron already voted on proposal {}, cannot vote again.", proposal_id))
            }
            _err => Err(anyhow::anyhow!("Error registering vote: {:?}", _err)),
        }
    }

    pub async fn refresh_neuron(&self, neuron_id: u64) -> anyhow::Result<ManageNeuronResponse> {
        self.manage_neuron(&ManageNeuron {
            id: Some(NeuronId { id: neuron_id }),
            neuron_id_or_subaccount: None,
            command: Some(CoR(ClaimOrRefresh {
                by: Some(By::NeuronIdOrSubaccount(ic_nns_governance::pb::v1::Empty {})),
            })),
        })
        .await
    }

    async fn manage_neuron(&self, manage_neuron: &ManageNeuron) -> anyhow::Result<ManageNeuronResponse> {
        let resp = self
            .client
            .agent
            .update(&GOVERNANCE_CANISTER_ID.into(), "manage_neuron")
            .with_effective_canister_id(GOVERNANCE_CANISTER_ID.into())
            .with_arg(candid::encode_one(manage_neuron)?)
            .call_and_wait()
            .await
            .map_err(anyhow::Error::from)?;

        Decode!(resp.as_slice(), ManageNeuronResponse).map_err(anyhow::Error::from)
    }

    pub async fn make_proposal(&self, proposer_id: NeuronId, proposal: Proposal) -> anyhow::Result<MakeProposalResponse> {
        let mng = ManageNeuron {
            id: None,
            neuron_id_or_subaccount: Some(NeuronIdOrSubaccount::NeuronId(proposer_id)),
            command: Some(Command::MakeProposal(proposal.into())),
        };
        let resp = self.manage_neuron(&mng).await?;
        match resp.command {
            None => Err(anyhow::anyhow!("No command in response")),
            Some(CommandResponse::MakeProposal(resp)) => Ok(resp),
            Some(CommandResponse::Error(resp)) => Err(anyhow::anyhow!("{:?}", resp)),
            Some(_) => Err(anyhow::anyhow!("Unexpected command response to proposal request: {:?}", resp.command)),
        }
    }

    pub async fn list_proposals(&self, contract: ListProposalInfoRequest) -> anyhow::Result<Vec<ProposalInfo>> {
        self.query::<ListProposalInfoResponse>("list_proposals", candid::encode_one(&contract)?)
            .await
            .map(|r| r.proposal_info)
    }

    pub async fn get_neuron_info(&self, neuron_id: u64) -> anyhow::Result<NeuronInfo> {
        self.query::<Result<NeuronInfo, GovernanceError>>("get_neuron_info", candid::encode_one(neuron_id)?)
            .await?
            .map_err(|e| anyhow::anyhow!("Failed to read neuron {}: {:?}", neuron_id, e))
    }

    pub async fn get_full_neuron(&self, neuron_id: u64) -> anyhow::Result<Neuron> {
        self.query::<Result<Neuron, GovernanceError>>("get_full_neuron", candid::encode_one(neuron_id)?)
            .await?
            .map_err(|e| anyhow::anyhow!("Failed to get full neuron {}: {:?}", neuron_id, e))
    }

    pub async fn list_neurons(&self) -> anyhow::Result<ListNeuronsResponse> {
        let mut page_number = 0;
        let mut acc = ListNeuronsResponse::default();
        loop {
            let current = self.list_neurons_inner(page_number).await?;
            acc.full_neurons.extend(current.full_neurons);
            acc.neuron_infos.extend(current.neuron_infos);

            if Some(page_number) >= acc.total_pages_available || acc.total_pages_available.is_none() {
                break;
            }

            page_number += 1;
        }

        Ok(acc)
    }

    async fn list_neurons_inner(&self, page: u64) -> anyhow::Result<ListNeuronsResponse> {
        self.query(
            "list_neurons",
            candid::encode_one(ListNeurons {
                neuron_ids: vec![],
                include_neurons_readable_by_caller: true,
                include_empty_neurons_readable_by_caller: None,
                include_public_neurons_in_full_neurons: None,
                page_number: Some(page),
                page_size: None,
                neuron_subaccounts: None,
            })?,
        )
        .await
    }

    pub async fn get_node_providers(&self) -> anyhow::Result<Vec<PbNodeProvider>> {
        let response = self
            .query::<ListNodeProvidersResponse>("list_node_providers", candid::encode_one(())?)
            .await?;
        let node_providers = response.node_providers.into_iter().map(PbNodeProvider::from).collect();
        Ok(node_providers)
    }

    pub async fn list_node_provider_rewards(&self, date_filter: Option<DateRangeFilter>) -> anyhow::Result<Vec<MonthlyNodeProviderRewards>> {
        let response = self
            .query::<ListNodeProviderRewardsResponse>(
                "list_node_provider_rewards",
                candid::encode_one(ListNodeProviderRewardsRequest { date_filter })?,
            )
            .await?;
        Ok(response.rewards)
    }

    pub async fn get_node_provider_rewards(&self) -> anyhow::Result<MonthlyNodeProviderRewards> {
        let resp = self
            .client
            .agent
            .update(&GOVERNANCE_CANISTER_ID.into(), "get_node_provider_rewards")
            .with_effective_canister_id(GOVERNANCE_CANISTER_ID.into())
            .call_and_wait()
            .await
            .map_err(anyhow::Error::from)?;

        let result: Result<MonthlyNodeProviderRewards, GovernanceError> =
            Decode!(resp.as_slice(), Result<MonthlyNodeProviderRewards, GovernanceError>).map_err(anyhow::Error::from)?;
        result.map_err(|e| anyhow::anyhow!("Governance error: {:?}", e))
    }

    async fn query<T>(&self, method_name: &str, args: Vec<u8>) -> anyhow::Result<T>
    where
        T: candid::CandidType + for<'a> candid::Deserialize<'a>,
    {
        self.client.query(&GOVERNANCE_CANISTER_ID.into(), method_name, args).await
    }
}
