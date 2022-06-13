use std::convert::TryFrom;
use std::str::FromStr;

use anyhow::Result;
use candid::{CandidType, Decode, Encode};
use ic_agent::Agent;
use ic_base_types::PrincipalId;
use ic_nns_governance::pb::v1::{proposal::Action, ListProposalInfo, ListProposalInfoResponse, NnsFunction};
use ic_nns_governance::pb::v1::{ProposalInfo, ProposalStatus};
use lazy_static::lazy_static;
use mercury_management_types::{
    CreateSubnetProposalInfo, ReplaceNodeProposalInfo, TopologyProposal, TopologyProposalKind, TopologyProposalStatus,
};
use regex::Regex;
use registry_canister::mutations::do_add_nodes_to_subnet::AddNodesToSubnetPayload;
use registry_canister::mutations::do_create_subnet::CreateSubnetPayload;
use registry_canister::mutations::do_remove_nodes_from_subnet::RemoveNodesFromSubnetPayload;
use registry_canister::mutations::do_update_subnet_replica::UpdateSubnetReplicaVersionPayload;
use serde::Serialize;

pub struct ProposalAgent {
    agent: Agent,
}

pub trait NnsFunctionProposal: CandidType + serde::de::DeserializeOwned {
    const TYPE: NnsFunction;
    fn decode(function_type: NnsFunction, function_payload: &[u8]) -> Result<Self> {
        if function_type == Self::TYPE {
            Decode!(function_payload, Self).map_err(|e| anyhow::format_err!("failed decoding candid: {}", e))
        } else {
            Err(anyhow::format_err!("unsupported NNS function"))
        }
    }
}

impl NnsFunctionProposal for AddNodesToSubnetPayload {
    const TYPE: NnsFunction = NnsFunction::AddNodeToSubnet;
}

impl NnsFunctionProposal for RemoveNodesFromSubnetPayload {
    const TYPE: NnsFunction = NnsFunction::RemoveNodesFromSubnet;
}

impl NnsFunctionProposal for CreateSubnetPayload {
    const TYPE: NnsFunction = NnsFunction::CreateSubnet;
}

impl NnsFunctionProposal for UpdateSubnetReplicaVersionPayload {
    const TYPE: NnsFunction = NnsFunction::UpdateSubnetReplicaVersion;
}

trait MoveNodesProposalPayload: NnsFunctionProposal {
    fn get_nodes(&self) -> Vec<PrincipalId>;
}

impl MoveNodesProposalPayload for AddNodesToSubnetPayload {
    fn get_nodes(&self) -> Vec<PrincipalId> {
        self.node_ids.iter().map(|node_id| node_id.get()).collect()
    }
}

impl MoveNodesProposalPayload for RemoveNodesFromSubnetPayload {
    fn get_nodes(&self) -> Vec<PrincipalId> {
        self.node_ids.iter().map(|node_id| node_id.get()).collect()
    }
}

// Copied so it can be serialized
#[derive(Clone, Serialize, Debug)]
pub struct ProposalInfoInternal {
    pub id: u64,
    pub proposal_timestamp_seconds: u64,
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
            executed_timestamp_seconds: _,
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

    fn filter_map_node_move_proposals<T: MoveNodesProposalPayload>(
        proposals: Vec<(ProposalInfo, T)>,
    ) -> Vec<(ProposalInfo, TopologyProposalKind)> {
        proposals
            .into_iter()
            .filter_map(|(proposal, payload)| {
                let summary = proposal.proposal.as_ref().expect("proposal is empty").summary.clone();

                if summary.contains("# Replace") {
                    lazy_static! {
                        static ref NODE_ID_GROUP: &'static str = "node_id";
                        static ref STEP_DETAILS_GROUP: &'static str = "step_detail";
                        static ref RE_ADD_NODE: Regex = Regex::new(&format!(
                            r#"\((?P<{}>.+)\): Add nodes \[(?P<{}>[^\]]+)\]"#,
                            *STEP_DETAILS_GROUP, *NODE_ID_GROUP,
                        ))
                        .unwrap();
                        static ref RE_REMOVE_NODE: Regex =
                            Regex::new(&format!(r#"Remove nodes \[(?P<{}>[^\]]+)\]"#, *NODE_ID_GROUP,)).unwrap();
                    }

                    let add_nodes_capture = RE_ADD_NODE.captures(&summary);

                    let first = add_nodes_capture
                        .as_ref()
                        .and_then(|c| c.name(&STEP_DETAILS_GROUP))
                        .map(|m| m.as_str() == "this proposal");
                    let new_nodes = add_nodes_capture.and_then(|c| c.name(&NODE_ID_GROUP)).and_then(|m| {
                        let nodes = m
                            .as_str()
                            .split(", ")
                            .map(|s| PrincipalId::from_str(s).ok())
                            .collect::<Vec<_>>();
                        if nodes.iter().any(|n| n.is_none()) {
                            None
                        } else {
                            Some(nodes.into_iter().map(|n| n.unwrap()).collect::<Vec<_>>())
                        }
                    });

                    let old_nodes = RE_REMOVE_NODE
                        .captures(&summary)
                        .and_then(|c| c.name(&NODE_ID_GROUP))
                        .and_then(|m| {
                            let nodes = m
                                .as_str()
                                .split(", ")
                                .map(|s| PrincipalId::from_str(s).ok())
                                .collect::<Vec<_>>();
                            if nodes.iter().any(|n| n.is_none()) {
                                None
                            } else {
                                Some(nodes.into_iter().map(|n| n.unwrap()).collect::<Vec<_>>())
                            }
                        });

                    new_nodes
                        .and_then(|new_nodes| old_nodes.map(|old_nodes| (new_nodes, old_nodes)))
                        .and_then(|(new_nodes, old_nodes)| first.map(|first| (new_nodes, old_nodes, first)))
                        .map(|(new_nodes, old_nodes, first)| {
                            (
                                proposal,
                                TopologyProposalKind::ReplaceNode(ReplaceNodeProposalInfo {
                                    new_nodes,
                                    old_nodes,
                                    first,
                                }),
                            )
                        })
                } else if summary.contains("# Cancel replacement") && T::TYPE == NnsFunction::RemoveNodesFromSubnet {
                    (
                        proposal,
                        TopologyProposalKind::ReplaceNode(ReplaceNodeProposalInfo {
                            new_nodes: payload.get_nodes(),
                            old_nodes: vec![],
                            first: false,
                        }),
                    )
                        .into()
                } else {
                    // TODO: extend subnet
                    None
                }
            })
            .collect()
    }

    pub async fn list_valid_topology_proposals(&self) -> Result<Vec<TopologyProposal>> {
        let proposals = &self.list_proposals().await?;
        let create_subnet_proposals = filter_map_nns_function_proposals::<CreateSubnetPayload>(proposals)
            .into_iter()
            .map(|(proposal, payload)| {
                (
                    proposal,
                    TopologyProposalKind::CreateSubnet(CreateSubnetProposalInfo {
                        nodes: payload.node_ids.into_iter().map(|node_id| node_id.get()).collect(),
                    }),
                )
            });

        let add_nodes_proposals = Self::filter_map_node_move_proposals(filter_map_nns_function_proposals::<
            AddNodesToSubnetPayload,
        >(proposals));

        let remove_nodes_proposals = Self::filter_map_node_move_proposals(filter_map_nns_function_proposals::<
            RemoveNodesFromSubnetPayload,
        >(proposals));

        let mut result = create_subnet_proposals
            .chain(add_nodes_proposals)
            .chain(remove_nodes_proposals)
            .collect::<Vec<_>>();
        result.sort_by_key(|(proposal, _)| proposal.id.expect("proposal id is missing").id);
        result.reverse();
        let result = result;

        Ok(result
            .into_iter()
            .filter_map(|(p, kind)| {
                ProposalStatus::from_i32(p.status)
                    .and_then(|status| TopologyProposalStatus::try_from(status).ok())
                    .map(|status| TopologyProposal {
                        id: p.id.expect("proposal id is missing").id,
                        status,
                        kind,
                    })
            })
            .collect())
    }

    pub async fn list_update_subnet_version_proposals(&self) -> Result<Vec<SubnetUpdateProposal>> {
        Ok(filter_map_nns_function_proposals(&self.list_proposals().await?)
            .into_iter()
            .map(|(info, payload)| SubnetUpdateProposal {
                info: info.into(),
                payload,
            })
            .collect::<Vec<_>>())
    }

    async fn list_proposals(&self) -> Result<Vec<ProposalInfo>> {
        Decode!(
            self.agent
                .query(
                    &ic_types_old::Principal::from_slice(ic_nns_constants::GOVERNANCE_CANISTER_ID.get().as_slice(),),
                    "list_proposals",
                )
                .with_arg(
                    Encode!(&ListProposalInfo {
                        limit: 1000,
                        exclude_topic: vec![0, 1, 2, 3, 4, 5, 6, 8, 9, 10],
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
