use std::time::Duration;

use anyhow::Result;
use backon::ExponentialBuilder;
use backon::Retryable;
use candid::{Decode, Encode};
use futures::future::BoxFuture;
use futures_util::future::try_join_all;
use ic_agent::Agent;
use ic_management_types::filter_map_nns_function_proposals;
use ic_management_types::UpdateElectedHostosVersionsProposal;
use ic_management_types::UpdateElectedReplicaVersionsProposal;
use ic_management_types::UpdateNodesHostosVersionsProposal;
use ic_management_types::{TopologyChangePayload, TopologyChangeProposal};
use ic_nns_governance::pb::v1::{ListProposalInfo, ProposalStatus, Topic};
use ic_nns_governance_api::proposal::Action;
use ic_nns_governance_api::{ListProposalInfoResponse, ProposalInfo};
use itertools::Itertools;
use mockall::automock;
use registry_canister::mutations::do_add_nodes_to_subnet::AddNodesToSubnetPayload;
use registry_canister::mutations::do_change_subnet_membership::ChangeSubnetMembershipPayload;
use registry_canister::mutations::do_create_subnet::CreateSubnetPayload;
use registry_canister::mutations::do_deploy_guestos_to_all_subnet_nodes::DeployGuestosToAllSubnetNodesPayload;
use registry_canister::mutations::do_remove_nodes_from_subnet::RemoveNodesFromSubnetPayload;
use registry_canister::mutations::do_revise_elected_replica_versions::ReviseElectedGuestosVersionsPayload;
use registry_canister::mutations::do_update_elected_hostos_versions::UpdateElectedHostosVersionsPayload;
use registry_canister::mutations::do_update_nodes_hostos_version::UpdateNodesHostosVersionPayload;
use registry_canister::mutations::do_update_unassigned_nodes_config::UpdateUnassignedNodesConfigPayload;
use registry_canister::mutations::node_management::do_remove_nodes::RemoveNodesPayload;
use serde::Serialize;
use url::Url;

#[allow(dead_code)]
#[automock]
pub trait ProposalAgent: Send + Sync {
    fn list_open_topology_proposals(&self) -> BoxFuture<'_, Result<Vec<TopologyChangeProposal>>>;

    fn list_open_elect_replica_proposals(&self) -> BoxFuture<'_, Result<Vec<UpdateElectedReplicaVersionsProposal>>>;

    fn list_open_elect_hostos_proposals(&self) -> BoxFuture<'_, Result<Vec<UpdateElectedHostosVersionsProposal>>>;

    fn list_open_update_nodes_hostos_versions_proposals(&self) -> BoxFuture<'_, Result<Vec<UpdateNodesHostosVersionsProposal>>>;

    fn list_update_subnet_version_proposals(&self) -> BoxFuture<'_, Result<Vec<SubnetUpdateProposal>>>;

    fn list_update_unassigned_nodes_version_proposals(&self) -> BoxFuture<'_, Result<Vec<UpdateUnassignedNodesProposal>>>;
}

#[derive(Clone)]
pub struct ProposalAgentImpl {
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
            total_potential_voting_power: _,
        } = p;
        ProposalInfoInternal {
            id: id.expect("missing proposal id").id,
            proposal_timestamp_seconds,
            executed_timestamp_seconds,
            executed: ProposalStatus::try_from(status).expect("unknown status") == ProposalStatus::Executed,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct SubnetUpdateProposal {
    pub info: ProposalInfoInternal,
    pub payload: DeployGuestosToAllSubnetNodesPayload,
}

#[derive(Clone, Serialize)]
pub struct UpdateUnassignedNodesProposal {
    pub info: ProposalInfoInternal,
    pub payload: UpdateUnassignedNodesConfigPayload,
}

impl ProposalAgent for ProposalAgentImpl {
    fn list_open_topology_proposals(&self) -> BoxFuture<'_, Result<Vec<TopologyChangeProposal>>> {
        Box::pin(async {
            let proposals = &self.list_proposals(vec![ProposalStatus::Open]).await?;
            let create_subnet_proposals = Self::nodes_proposals(filter_map_nns_function_proposals::<CreateSubnetPayload>(proposals)).into_iter();

            let add_nodes_to_subnet_proposals =
                Self::nodes_proposals(filter_map_nns_function_proposals::<AddNodesToSubnetPayload>(proposals)).into_iter();

            let remove_nodes_from_subnet_proposals =
                Self::nodes_proposals(filter_map_nns_function_proposals::<RemoveNodesFromSubnetPayload>(proposals)).into_iter();

            let remove_nodes_proposals = Self::nodes_proposals(filter_map_nns_function_proposals::<RemoveNodesPayload>(proposals)).into_iter();

            let membership_change_proposals =
                Self::nodes_proposals(filter_map_nns_function_proposals::<ChangeSubnetMembershipPayload>(proposals)).into_iter();

            let mut result = create_subnet_proposals
                .chain(add_nodes_to_subnet_proposals)
                .chain(remove_nodes_from_subnet_proposals)
                .chain(membership_change_proposals)
                .chain(remove_nodes_proposals)
                .collect::<Vec<_>>();
            result.sort_by_key(|p| p.id);
            result.reverse();

            Ok(result)
        })
    }

    fn list_open_elect_replica_proposals(&self) -> BoxFuture<'_, Result<Vec<UpdateElectedReplicaVersionsProposal>>> {
        Box::pin(async {
            let proposals = &self.list_proposals(vec![ProposalStatus::Open]).await?;
            let open_elect_guest_proposals = filter_map_nns_function_proposals::<ReviseElectedGuestosVersionsPayload>(proposals);

            let result = open_elect_guest_proposals
                .into_iter()
                .map(|(proposal_info, proposal_payload)| UpdateElectedReplicaVersionsProposal {
                    proposal_id: proposal_info.id.expect("proposal should have an id").id,
                    version_elect: proposal_payload.replica_version_to_elect.expect("version elect should exist"),

                    versions_unelect: proposal_payload.replica_versions_to_unelect,
                })
                .sorted_by_key(|p| p.proposal_id)
                .rev()
                .collect::<Vec<_>>();

            Ok(result)
        })
    }

    fn list_open_elect_hostos_proposals(&self) -> BoxFuture<'_, Result<Vec<UpdateElectedHostosVersionsProposal>>> {
        Box::pin(async {
            let proposals = &self.list_proposals(vec![ProposalStatus::Open]).await?;
            let open_elect_guest_proposals = filter_map_nns_function_proposals::<UpdateElectedHostosVersionsPayload>(proposals);

            let result = open_elect_guest_proposals
                .into_iter()
                .map(|(proposal_info, proposal_payload)| UpdateElectedHostosVersionsProposal {
                    proposal_id: proposal_info.id.expect("proposal should have an id").id,
                    version_elect: proposal_payload.hostos_version_to_elect.expect("version elect should exist"),

                    versions_unelect: proposal_payload.hostos_versions_to_unelect,
                })
                .sorted_by_key(|p| p.proposal_id)
                .rev()
                .collect::<Vec<_>>();

            Ok(result)
        })
    }

    fn list_open_update_nodes_hostos_versions_proposals(&self) -> BoxFuture<'_, Result<Vec<UpdateNodesHostosVersionsProposal>>> {
        Box::pin(async {
            let proposals = &self.list_proposals(vec![ProposalStatus::Open]).await?;
            let open_update_nodes_hostos_versions_proposals = filter_map_nns_function_proposals::<UpdateNodesHostosVersionPayload>(proposals);

            let result = open_update_nodes_hostos_versions_proposals
                .into_iter()
                .map(|(proposal_info, proposal_payload)| UpdateNodesHostosVersionsProposal {
                    proposal_id: proposal_info.id.expect("proposal should have an id").id,
                    hostos_version_id: proposal_payload.hostos_version_id.expect("version elect should exist"),

                    node_ids: proposal_payload.node_ids,
                })
                .sorted_by_key(|p| p.proposal_id)
                .rev()
                .collect::<Vec<_>>();

            Ok(result)
        })
    }

    fn list_update_subnet_version_proposals(&self) -> BoxFuture<'_, Result<Vec<SubnetUpdateProposal>>> {
        Box::pin(async {
            Ok(filter_map_nns_function_proposals(&self.list_proposals(vec![]).await?)
                .into_iter()
                .map(|(info, payload)| SubnetUpdateProposal { info: info.into(), payload })
                .collect::<Vec<_>>())
        })
    }

    fn list_update_unassigned_nodes_version_proposals(&self) -> BoxFuture<'_, Result<Vec<UpdateUnassignedNodesProposal>>> {
        Box::pin(async {
            Ok(filter_map_nns_function_proposals(&self.list_proposals(vec![]).await?)
                .into_iter()
                .map(|(info, payload)| UpdateUnassignedNodesProposal { info: info.into(), payload })
                .collect::<Vec<_>>())
        })
    }
}

#[allow(dead_code)]
impl ProposalAgentImpl {
    pub fn new(nns_urls: &[Url]) -> Self {
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Could not create HTTP client.");
        let agent = Agent::builder()
            .with_http_client(client)
            .with_url(nns_urls[0].clone())
            .with_verify_query_signatures(false)
            .build()
            .expect("failed to build the agent");

        Self { agent }
    }

    fn nodes_proposals<T: TopologyChangePayload>(proposals: Vec<(ProposalInfo, T)>) -> Vec<TopologyChangeProposal> {
        proposals.into_iter().map(TopologyChangeProposal::from).collect()
    }

    async fn list_proposals(&self, include_status: Vec<ProposalStatus>) -> Result<Vec<ProposalInfo>> {
        let mut proposals = vec![];
        loop {
            let fetch_partial_results = || async {
                let f = self
                    .agent
                    .query(
                        &ic_agent::export::Principal::from_slice(ic_nns_constants::GOVERNANCE_CANISTER_ID.get().as_slice()),
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
                                Topic::ApplicationCanisterManagement,
                                Topic::Kyc,
                                Topic::NodeProviderRewards,
                                Topic::SnsAndCommunityFund,
                                // Topic::SubnetReplicaVersionManagement,
                                // Topic::ReplicaVersionManagement,
                                Topic::SnsAndCommunityFund,
                            ]
                            .into_iter()
                            .map(|t| t.into())
                            .collect(),
                            include_status: include_status.clone().into_iter().map(|s| s.into()).collect(),
                            before_proposal: proposals.last().map(|p: &ProposalInfo| p.id.expect("proposal should have an id")),
                            ..Default::default()
                        })
                        .expect("encode failed"),
                    )
                    .call();
                Decode!(f.await?.as_slice(), ListProposalInfoResponse)
                    .map(|lp| lp.proposal_info)
                    .map_err(|e| anyhow::format_err!("failed to decode list proposals: {}", e))
            };
            let partial_result = fetch_partial_results.retry(ExponentialBuilder::default()).await?;
            if partial_result.is_empty() {
                break;
            } else {
                proposals.extend(partial_result);
            }
        }
        let (empty_payload_proposals, full_payload_proposals): (_, Vec<_>) = proposals.into_iter().partition(|p| {
            if let Some(Action::ExecuteNnsFunction(action)) = p.proposal.clone().expect("proposal is not empty").action {
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
                        &ic_agent::export::Principal::from_slice(ic_nns_constants::GOVERNANCE_CANISTER_ID.get().as_slice()),
                        "get_proposal_info",
                    )
                    .with_arg(Encode!(&id).expect("encode failed"))
                    .call();
                Decode!(f.await?.as_slice(), Option<ProposalInfo>).map_err(|e| anyhow::format_err!("failed to decode list proposals: {}", e))
            };
            fetch_partial_results.retry(ExponentialBuilder::default()).await
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
