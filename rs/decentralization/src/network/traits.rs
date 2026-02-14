use log::info;

use super::*;

pub trait AvailableNodesQuerier {
    fn available_nodes(&self) -> BoxFuture<'_, Result<Vec<Node>, NetworkError>>;
}

pub trait NodesConverter {
    fn get_nodes<'a>(&'a self, from: &'a [PrincipalId]) -> BoxFuture<'a, Result<Vec<Node>, NetworkError>>;
}

#[derive(Clone)]
pub enum SubnetQueryBy {
    SubnetId(PrincipalId),
    NodeList(Vec<Node>),
}

pub trait SubnetQuerier {
    fn subnet(&self, by: SubnetQueryBy) -> BoxFuture<'_, Result<DecentralizedSubnet, NetworkError>>;
}

pub trait TopologyManager: SubnetQuerier + AvailableNodesQuerier + Sync {
    fn modify_subnet_nodes(&self, by: SubnetQueryBy) -> BoxFuture<'_, Result<SubnetChangeRequest, NetworkError>> {
        Box::pin(async {
            Ok(SubnetChangeRequest {
                available_nodes: self.available_nodes().await?,
                subnet: self.subnet(by).await?,
                ..Default::default()
            })
        })
    }

    fn create_subnet<'a>(
        &'a self,
        size: usize,
        add_nodes: Vec<PrincipalId>,
        exclude_nodes: Vec<String>,
        only_nodes: Vec<String>,
        health_of_nodes: &'a IndexMap<PrincipalId, HealthStatus>,
        cordoned_features: Vec<CordonedFeature>,
        all_nodes: &'a [Node],
    ) -> BoxFuture<'a, Result<SubnetChange, NetworkError>> {
        Box::pin(async move {
            let mut available_nodes = self.available_nodes().await?;
            let cache_added_nodes: Vec<_> = available_nodes.iter().filter(|n| add_nodes.iter().match_any(n)).cloned().collect();
            if !only_nodes.is_empty() {
                info!("`only` nodes provided. Overriding the pool of available nodes");
                available_nodes.retain(|n| only_nodes.iter().match_any(n));
            }
            // To allow for more intuitive usage.
            //
            // The user requests a subnet of size n and provides some initial nodes
            // (maybe just m where m is less than n) and the tool selects the
            // remaining nodes from the pool of available nodes.
            let size = if add_nodes.len() > size {
                panic!("Cannot make subnet of size {size} with {} nodes", add_nodes.len());
            } else {
                let remaining_for_tool_to_pick = size - add_nodes.len();
                if remaining_for_tool_to_pick > 0 {
                    info!(
                        "Provided {} nodes. The tool will pick remaining {remaining_for_tool_to_pick} to make a subnet with {size} nodes.",
                        add_nodes.len()
                    );
                    if available_nodes.len() < remaining_for_tool_to_pick {
                        panic!(
                            "The pool of available_nodes is too small to pick the remaining {remaining_for_tool_to_pick}. Remaning pool has {} nodes.",
                            available_nodes.len()
                        );
                    }
                    info!("Picking {remaining_for_tool_to_pick} from the pool of {} nodes", available_nodes.len());
                }
                remaining_for_tool_to_pick
            };

            // The tool needs to see the explicitly added nodes as available_nodes.
            //
            // The previous size logic would not show correct numbers if this was
            // done earlier.
            available_nodes.extend(cache_added_nodes);

            SubnetChangeRequest {
                available_nodes,
                ..Default::default()
            }
            .adding_from_available(add_nodes)
            .excluding_from_available(exclude_nodes)
            .resize(size, 0, 0, health_of_nodes, cordoned_features, all_nodes)
        })
    }
}

pub trait Identifies<Node> {
    fn eq(&self, other: &Node) -> bool;
    fn partial_eq(&self, other: &Node) -> bool;
}

impl Identifies<Node> for PrincipalId {
    fn eq(&self, other: &Node) -> bool {
        &other.principal == self
    }
    fn partial_eq(&self, other: &Node) -> bool {
        Identifies::eq(self, other)
    }
}

impl Identifies<Node> for String {
    fn eq(&self, other: &Node) -> bool {
        other.matches_feature_value(self)
    }
    fn partial_eq(&self, other: &Node) -> bool {
        Identifies::eq(self, other)
    }
}

impl Identifies<Node> for Node {
    fn eq(&self, other: &Node) -> bool {
        self == other
    }
    fn partial_eq(&self, other: &Node) -> bool {
        Identifies::eq(self, other)
    }
}

pub(crate) trait MatchAnyNode<T: Identifies<Node>> {
    fn match_any(self, node: &Node) -> bool;
}

impl<T: Identifies<Node>> MatchAnyNode<T> for std::slice::Iter<'_, T> {
    fn match_any(mut self, node: &Node) -> bool {
        self.any(|n| n.eq(node))
    }
}
