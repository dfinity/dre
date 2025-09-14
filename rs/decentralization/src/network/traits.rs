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
            SubnetChangeRequest {
                available_nodes: self.available_nodes().await?,
                ..Default::default()
            }
            .adding_from_available(add_nodes)
            .excluding_from_available(exclude_nodes)
            .adding_from_available(only_nodes)
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
