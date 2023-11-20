use anyhow::Result;
use backon::ExponentialBuilder;
use backon::Retryable;
use candid::{Decode, Encode};

use futures_util::future::try_join_all;
use ic_agent::Agent;
use ic_management_types::{NnsFunctionProposal, TopologyChangePayload, TopologyChangeProposal};
use ic_management_types::{UpdateElectedHostosVersionsProposal, UpdateElectedReplicaVersionsProposal};
use ic_nns_governance::pb::v1::{proposal::Action, ListProposalInfo, ListProposalInfoResponse, NnsFunction};
use ic_nns_governance::pb::v1::{ProposalInfo, ProposalStatus, Topic};
use itertools::Itertools;
use registry_canister::mutations::do_add_nodes_to_subnet::AddNodesToSubnetPayload;
use registry_canister::mutations::do_change_subnet_membership::ChangeSubnetMembershipPayload;
use registry_canister::mutations::do_create_subnet::CreateSubnetPayload;
use registry_canister::mutations::do_remove_nodes_from_subnet::RemoveNodesFromSubnetPayload;
use registry_canister::mutations::do_update_elected_hostos_versions::UpdateElectedHostosVersionsPayload;
use registry_canister::mutations::do_update_elected_replica_versions::UpdateElectedReplicaVersionsPayload;
use registry_canister::mutations::do_update_subnet_replica::UpdateSubnetReplicaVersionPayload;
use registry_canister::mutations::node_management::do_remove_nodes::RemoveNodesPayload;
use serde::Serialize;

pub struct ProposalAgent {
    agent: Agent,
}

// Copied so it can be serialized
#[derive(Clone, Serialize, Debug)]
pub struct ProposalInfoInternal {
    pub id: u64,
    pub proposal_timestamp_seconds: u64,
    pub executed_timestamp_seconds: u64,
    pub executed: bool,
}

impl From<ProposalInfo> for ProposalInfoInternal {
    fn from(p: ProposalInfo) -> Self {
        let ProposalInfo {
            id,
            proposer: _,
            reject_cost_e8s: _,
            proposal: _,
            proposal_timestamp_seconds,
            ballots: _,
            latest_tally: _,
            decided_timestamp_seconds: _,
            executed_timestamp_seconds,
            failed_timestamp_seconds: _,
            failure_reason: _,
            reward_event_round: _,
            topic: _,
            status,
            reward_status: _,
            deadline_timestamp_seconds: _,
            derived_proposal_information: _,
        } = p;
        ProposalInfoInternal {
            id: id.expect("missing proposal id").id,
            proposal_timestamp_seconds,
            executed_timestamp_seconds,
            executed: ProposalStatus::from_i32(status).expect("unknown status") == ProposalStatus::Executed,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct SubnetUpdateProposal {
    pub info: ProposalInfoInternal,
    pub payload: UpdateSubnetReplicaVersionPayload,
}

impl ProposalAgent {
    pub fn new(url: String) -> Self {
        let agent = Agent::builder()
            .with_transport(
                ic_agent::agent::http_transport::ReqwestHttpReplicaV2Transport::create(url)
                    .expect("failed to create transport"),
            )
            .build()
            .expect("failed to build the agent");
        Self { agent }
    }

    fn nodes_proposals<T: TopologyChangePayload>(proposals: Vec<(ProposalInfo, T)>) -> Vec<TopologyChangeProposal> {
        proposals.into_iter().map(TopologyChangeProposal::from).collect()
    }

    pub async fn list_open_topology_proposals(&self) -> Result<Vec<TopologyChangeProposal>> {
        let proposals = &self.list_proposals(vec![ProposalStatus::Open]).await?;
        let create_subnet_proposals =
            Self::nodes_proposals(filter_map_nns_function_proposals::<CreateSubnetPayload>(proposals)).into_iter();

        let add_nodes_to_subnet_proposals =
            Self::nodes_proposals(filter_map_nns_function_proposals::<AddNodesToSubnetPayload>(proposals)).into_iter();

        let remove_nodes_from_subnet_proposals = Self::nodes_proposals(filter_map_nns_function_proposals::<
            RemoveNodesFromSubnetPayload,
        >(proposals))
        .into_iter();

        let remove_nodes_proposals =
            Self::nodes_proposals(filter_map_nns_function_proposals::<RemoveNodesPayload>(proposals)).into_iter();

        let membership_change_proposals = Self::nodes_proposals(filter_map_nns_function_proposals::<
            ChangeSubnetMembershipPayload,
        >(proposals))
        .into_iter();

        let mut result = create_subnet_proposals
            .chain(add_nodes_to_subnet_proposals)
            .chain(remove_nodes_from_subnet_proposals)
            .chain(membership_change_proposals)
            .chain(remove_nodes_proposals)
            .collect::<Vec<_>>();
        result.sort_by_key(|p| p.id);
        result.reverse();

        Ok(result)
    }

    pub async fn list_open_elect_replica_proposals(&self) -> Result<Vec<UpdateElectedReplicaVersionsProposal>> {
        let proposals = &self.list_proposals(vec![ProposalStatus::Open]).await?;
        let open_elect_guest_proposals =
            filter_map_nns_function_proposals::<UpdateElectedReplicaVersionsPayload>(proposals);

        let result = open_elect_guest_proposals
            .into_iter()
            .map(
                |(proposal_info, proposal_payload)| UpdateElectedReplicaVersionsProposal {
                    proposal_id: proposal_info.id.expect("proposal should have an id").id,
                    version_elect: proposal_payload
                        .replica_version_to_elect
                        .expect("version elect should exist"),

                    versions_unelect: proposal_payload.replica_versions_to_unelect,
                },
            )
            .sorted_by_key(|p| p.proposal_id)
            .rev()
            .collect::<Vec<_>>();

        Ok(result)
    }

    pub async fn list_open_elect_hostos_proposals(&self) -> Result<Vec<UpdateElectedHostosVersionsProposal>> {
        let proposals = &self.list_proposals(vec![ProposalStatus::Open]).await?;
        let open_elect_guest_proposals =
            filter_map_nns_function_proposals::<UpdateElectedHostosVersionsPayload>(proposals);

        let result = open_elect_guest_proposals
            .into_iter()
            .map(
                |(proposal_info, proposal_payload)| UpdateElectedHostosVersionsProposal {
                    proposal_id: proposal_info.id.expect("proposal should have an id").id,
                    version_elect: proposal_payload
                        .hostos_version_to_elect
                        .expect("version elect should exist"),

                    versions_unelect: proposal_payload.hostos_versions_to_unelect,
                },
            )
            .sorted_by_key(|p| p.proposal_id)
            .rev()
            .collect::<Vec<_>>();

        Ok(result)
    }

    pub async fn list_update_subnet_version_proposals(&self) -> Result<Vec<SubnetUpdateProposal>> {
        Ok(filter_map_nns_function_proposals(&self.list_proposals(vec![]).await?)
            .into_iter()
            .map(|(info, payload)| SubnetUpdateProposal {
                info: info.into(),
                payload,
            })
            .collect::<Vec<_>>())
    }

    async fn list_proposals(&self, include_status: Vec<ProposalStatus>) -> Result<Vec<ProposalInfo>> {
        let mut proposals = vec![];
        loop {
            let fetch_partial_results = || async {
                let f = self
                    .agent
                    .query(
                        &ic_agent::export::Principal::from_slice(
                            ic_nns_constants::GOVERNANCE_CANISTER_ID.get().as_slice(),
                        ),
                        "list_proposals",
                    )
                    .with_arg(
                        Encode!(&ListProposalInfo {
                            limit: 1000,
                            // 0, 1, 2, 3, 4, 5, 6, 8, 9, 10
                            exclude_topic: vec![
                                Topic::Unspecified,
                                Topic::NeuronManagement,
                                Topic::ExchangeRate,
                                Topic::NetworkEconomics,
                                Topic::Governance,
                                // Topic::NodeAdmin,
                                Topic::ParticipantManagement,
                                // Topic::SubnetManagement,
                                Topic::NetworkCanisterManagement,
                                Topic::Kyc,
                                Topic::NodeProviderRewards,
                                Topic::SnsDecentralizationSale,
                                // Topic::SubnetReplicaVersionManagement,
                                // Topic::ReplicaVersionManagement,
                                Topic::SnsAndCommunityFund,
                            ]
                            .into_iter()
                            .map(|t| t.into())
                            .collect(),
                            include_status: include_status.clone().into_iter().map(|s| s.into()).collect(),
                            before_proposal: proposals
                                .last()
                                .map(|p: &ProposalInfo| p.id.expect("proposal should have an id")),
                            ..Default::default()
                        })
                        .expect("encode failed"),
                    )
                    .call();
                Decode!(f.await?.as_slice(), ListProposalInfoResponse)
                    .map(|lp| lp.proposal_info)
                    .map_err(|e| anyhow::format_err!("failed to decode list proposals: {}", e))
            };
            let partial_result = fetch_partial_results.retry(&ExponentialBuilder::default()).await?;
            if partial_result.is_empty() {
                break;
            } else {
                proposals.extend(partial_result);
            }
        }
        let (empty_payload_proposals, full_payload_proposals): (_, Vec<_>) = proposals.into_iter().partition(|p| {
            if let Some(Action::ExecuteNnsFunction(action)) = p.proposal.clone().expect("proposal is not empty").action
            {
                return action.payload.is_empty();
            }
            false
        });
        try_join_all(empty_payload_proposals.iter().map(|p| async {
            let id = p.id.expect("proposal should have id").id;
            let fetch_partial_results = || async {
                let f = self
                    .agent
                    .query(
                        &ic_agent::export::Principal::from_slice(
                            ic_nns_constants::GOVERNANCE_CANISTER_ID.get().as_slice(),
                        ),
                        "get_proposal_info",
                    )
                    .with_arg(Encode!(&id).expect("encode failed"))
                    .call();
                Decode!(f.await?.as_slice(), Option<ProposalInfo>)
                    .map_err(|e| anyhow::format_err!("failed to decode list proposals: {}", e))
            };
            fetch_partial_results.retry(&ExponentialBuilder::default()).await
        }))
        .await
        .map(|proposals| {
            let mut proposals = proposals.into_iter().flatten().collect::<Vec<_>>();
            proposals.extend(full_payload_proposals);
            proposals.sort_by_key(|p| p.id.expect("proposal id exists").id);
            proposals
        })
    }
}

fn filter_map_nns_function_proposals<T: NnsFunctionProposal + candid::CandidType>(
    proposals: &[ProposalInfo],
) -> Vec<(ProposalInfo, T)> {
    proposals
        .iter()
        .filter(|p| ProposalStatus::from_i32(p.status).expect("unknown proposal status") != ProposalStatus::Rejected)
        .filter_map(|p| {
            p.proposal
                .as_ref()
                .and_then(|p| p.action.as_ref())
                .ok_or_else(|| anyhow::format_err!("no action"))
                .and_then(|a| match a {
                    Action::ExecuteNnsFunction(function) => NnsFunction::from_i32(function.nns_function)
                        .map(|f| (f, function.payload.as_slice()))
                        .ok_or_else(|| anyhow::format_err!("unknown NNS function")),
                    _ => Err(anyhow::format_err!("not an NNS function")),
                })
                .and_then(|(function_type, function_payload)| {
                    if function_type == T::TYPE {
                        Decode!(function_payload, T).map_err(|e| anyhow::format_err!("failed decoding candid: {}", e))
                    } else {
                        Err(anyhow::format_err!("unsupported NNS function"))
                    }
                })
                .ok()
                .map(|payload| (p.clone(), payload))
        })
        .collect()
}
