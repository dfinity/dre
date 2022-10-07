use anyhow::Result;
use candid::{Decode, Encode};
use ic_agent::Agent;
use ic_management_types::{NnsFunctionProposal, TopologyProposal, TopologyProposalPayload};
use ic_nns_governance::pb::v1::{proposal::Action, ListProposalInfo, ListProposalInfoResponse, NnsFunction};
use ic_nns_governance::pb::v1::{ProposalInfo, ProposalStatus};
use registry_canister::mutations::do_add_nodes_to_subnet::AddNodesToSubnetPayload;
use registry_canister::mutations::do_change_subnet_membership::ChangeSubnetMembershipPayload;
use registry_canister::mutations::do_create_subnet::CreateSubnetPayload;
use registry_canister::mutations::do_remove_nodes_from_subnet::RemoveNodesFromSubnetPayload;
use registry_canister::mutations::do_update_subnet_replica::UpdateSubnetReplicaVersionPayload;
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

    fn nodes_proposals<T: TopologyProposalPayload>(proposals: Vec<(ProposalInfo, T)>) -> Vec<TopologyProposal> {
        proposals.into_iter().map(TopologyProposal::from).collect()
    }

    pub async fn list_open_topology_proposals(&self) -> Result<Vec<TopologyProposal>> {
        let proposals = &self.list_proposals(vec![ProposalStatus::Open]).await?;
        let create_subnet_proposals =
            Self::nodes_proposals(filter_map_nns_function_proposals::<CreateSubnetPayload>(proposals)).into_iter();

        let add_nodes_proposals =
            Self::nodes_proposals(filter_map_nns_function_proposals::<AddNodesToSubnetPayload>(proposals)).into_iter();

        let remove_nodes_proposals = Self::nodes_proposals(filter_map_nns_function_proposals::<
            RemoveNodesFromSubnetPayload,
        >(proposals))
        .into_iter();

        let membership_change_proposals = Self::nodes_proposals(filter_map_nns_function_proposals::<
            ChangeSubnetMembershipPayload,
        >(proposals))
        .into_iter();

        let mut result = create_subnet_proposals
            .chain(add_nodes_proposals)
            .chain(remove_nodes_proposals)
            .chain(membership_change_proposals)
            .collect::<Vec<_>>();
        result.sort_by_key(|p| p.id);
        result.reverse();

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
        Decode!(
            self.agent
                .query(
                    &ic_agent::export::Principal::from_slice(ic_nns_constants::GOVERNANCE_CANISTER_ID.get().as_slice(),),
                    "list_proposals",
                )
                .with_arg(
                    Encode!(&ListProposalInfo {
                        limit: 1000,
                        exclude_topic: vec![0, 1, 2, 3, 4, 5, 6, 8, 9, 10],
                        include_status: include_status.into_iter().map(|s| s.into()).collect(),
                        ..Default::default()
                    })
                    .expect("encode failed")
                )
                .call()
                .await?
                .as_slice(),
            ListProposalInfoResponse
        )
        .map(|lp| lp.proposal_info)
        .map_err(|e| anyhow::format_err!("failed to decode list proposals: {}", e))
    }
}

fn filter_map_nns_function_proposals<T: NnsFunctionProposal>(proposals: &[ProposalInfo]) -> Vec<(ProposalInfo, T)> {
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
