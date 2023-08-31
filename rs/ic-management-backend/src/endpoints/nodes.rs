use futures_util::future::try_join;
use ic_management_types::requests::{NodeRemoval, NodeRemovalReason, NodesRemoveRequest, NodesRemoveResponse};
use itertools::Itertools;

use super::*;
use crate::health;
use decentralization::network::Node as DecentralizationNode;

/// Finds all nodes that need to be removed from the network either because
/// they're offline or duplicated
#[post("/nodes/remove")]
async fn remove(
    request: web::Json<NodesRemoveRequest>,
    registry: web::Data<Arc<RwLock<RegistryState>>>,
) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    let health_client = health::HealthClient::new(registry.network());
    let nodes = registry.nodes_with_proposals();
    let healths = health_client.nodes();

    response_from_result(
        try_join(healths, nodes)
            .await
            .map(|(mut healths, nodes)| {
                nodes
                    .values()
                    .cloned()
                    .map(|n| {
                        let status = healths
                            .remove(&n.principal)
                            .unwrap_or(ic_management_types::Status::Unknown);
                        (n, status)
                    })
                    .filter(|(n, _)| n.proposal.is_none())
                    .filter_map(|(n, status)| {
                        if n.subnet_id.is_some() {
                            return None;
                        }

                        let decentralization_node = DecentralizationNode::from(&n);

                        if let Some(exclude) = request.exclude.as_ref() {
                            for exclude_feature in exclude {
                                if decentralization_node.matches_feature_value(exclude_feature) {
                                    return None;
                                }
                            }
                        }

                        if let Some(filter) = request
                            .extra_nodes_filter
                            .iter()
                            .find(|f| decentralization_node.matches_feature_value(f))
                        {
                            return Some(NodeRemoval {
                                node: n,
                                reason: NodeRemovalReason::MatchedFilter(filter.clone()),
                            });
                        }

                        if !request.no_auto {
                            if let Some(principal) = n.duplicates {
                                return Some(NodeRemoval {
                                    node: n,
                                    reason: NodeRemovalReason::Duplicates(principal),
                                });
                            }
                            if !matches!(status, ic_management_types::Status::Healthy) {
                                return Some(NodeRemoval {
                                    node: n,
                                    reason: NodeRemovalReason::Unhealthy(status),
                                });
                            }
                        }

                        None
                    })
                    .collect::<Vec<_>>()
            })
            .map(|nodes| NodesRemoveResponse {
                motivation: "\n".to_string()
                    + &nodes
                        .iter()
                        .map(|nr| match nr.reason {
                            ic_management_types::requests::NodeRemovalReason::Duplicates(_)
                            | ic_management_types::requests::NodeRemovalReason::Unhealthy(_) => {
                                "Removing unhealthy nodes from the network, for redeployment"
                            }
                            ic_management_types::requests::NodeRemovalReason::MatchedFilter(_) => {
                                request.motivation.as_str()
                            }
                        })
                        .unique()
                        .map(|m| format!(" * {m}"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                removals: nodes,
            }),
    )
}
