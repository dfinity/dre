use core::fmt;
use std::collections::HashSet;
use std::sync::Arc;

use anyhow::anyhow;
use anyhow::Ok;
use decentralization::{
    network::{DecentralizedSubnet, SubnetQueryBy},
    SubnetChangeResponse,
};
use ic_management_backend::health::HealthStatusQuerier;
use ic_management_backend::lazy_registry::LazyRegistry;
use ic_management_types::HealthStatus;
use ic_management_types::Node;
use ic_types::PrincipalId;
use indexmap::IndexMap;
use itertools::Itertools;
use log::{info, warn};

use crate::cordoned_feature_fetcher::CordonedFeatureFetcher;

#[derive(Clone)]
pub enum SubnetTarget {
    FromId(PrincipalId),
    FromNodesIds(Vec<PrincipalId>),
}

#[derive(Debug)]
pub enum SubnetManagerError {
    SubnetTargetNotProvided,
}
impl std::error::Error for SubnetManagerError {}

impl fmt::Display for SubnetManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SubnetManagerError::SubnetTargetNotProvided => write!(f, "Subnet target is None"),
        }
    }
}

pub struct SubnetManager {
    subnet_target: Option<SubnetTarget>,
    registry_instance: Arc<dyn LazyRegistry>,
    cordoned_features_fetcher: Arc<dyn CordonedFeatureFetcher>,
    health_client: Arc<dyn HealthStatusQuerier>,
}

impl SubnetManager {
    pub fn new(
        registry_instance: Arc<dyn LazyRegistry>,
        cordoned_features_fetcher: Arc<dyn CordonedFeatureFetcher>,
        health_client: Arc<dyn HealthStatusQuerier>,
    ) -> Self {
        Self {
            subnet_target: None,
            registry_instance,
            cordoned_features_fetcher,
            health_client,
        }
    }

    pub fn with_target(self, target: SubnetTarget) -> Self {
        Self {
            subnet_target: Some(target),
            ..self
        }
    }

    fn target(&self) -> anyhow::Result<SubnetTarget> {
        self.subnet_target
            .clone()
            .ok_or_else(|| anyhow!(SubnetManagerError::SubnetTargetNotProvided))
    }

    async fn unhealthy_nodes(&self, subnet: DecentralizedSubnet) -> anyhow::Result<Vec<(Node, HealthStatus)>> {
        let subnet_health = self.health_client.subnet(subnet.id).await?;

        let unhealthy = subnet
            .nodes
            .into_iter()
            .filter_map(|n| match subnet_health.get(&n.principal) {
                Some(health) => {
                    if *health == HealthStatus::Healthy {
                        None
                    } else {
                        info!("Node {} is {:?}", n.id_short(), health);
                        Some((n, health.clone()))
                    }
                }
                None => {
                    warn!("Node {} has no known health, assuming unhealthy", n.id_short());
                    Some((n, HealthStatus::Unknown))
                }
            })
            .collect::<Vec<_>>();
        Ok(unhealthy)
    }

    async fn get_subnet_query_by(&self, target: SubnetTarget) -> anyhow::Result<SubnetQueryBy> {
        let converted = match target {
            SubnetTarget::FromId(id) => SubnetQueryBy::SubnetId(id),
            SubnetTarget::FromNodesIds(nodes) => {
                let nodes = self.registry_instance.get_nodes(&nodes).await?;
                SubnetQueryBy::NodeList(nodes)
            }
        };
        Ok(converted)
    }

    /// Simulates replacement of nodes in a subnet.
    /// There are multiple ways to replace nodes. For instance:
    ///    1. Setting `heal` to `true` in the request to replace unhealthy nodes
    ///    2. Replace `optimize` nodes to optimize subnet decentralization.
    ///    3. Explicitly add or remove nodes from the subnet specifying their
    ///       Principals.
    ///
    /// All nodes in the request must belong to exactly one subnet.
    pub async fn membership_replace(
        &self,
        heal: bool,
        motivation: Option<String>,
        optimize: Option<usize>,
        exclude: Option<Vec<String>>,
        only: Vec<String>,
        include: Option<Vec<PrincipalId>>,
        all_nodes: &[Node],
    ) -> anyhow::Result<SubnetChangeResponse> {
        let subnet_query_by = self.get_subnet_query_by(self.target()?).await?;
        let mut motivations = vec![];
        let mut to_be_replaced: Vec<Node> = if let SubnetQueryBy::NodeList(nodes) = &subnet_query_by {
            nodes.clone()
        } else {
            vec![]
        };

        let subnet_change_request = self
            .registry_instance
            .modify_subnet_nodes(subnet_query_by.clone())
            .await?
            .excluding_from_available(exclude.clone().unwrap_or_default())
            .including_from_available(only.clone())
            .including_from_available(include.clone().unwrap_or_default());

        let mut node_ids_unhealthy = HashSet::new();
        if heal {
            for (node, health_status) in self.unhealthy_nodes(subnet_change_request.subnet()).await? {
                node_ids_unhealthy.insert(node.principal);
                motivations.push(format!("replacing {} as it is unhealthy: {:?}", node.principal, health_status));
                to_be_replaced.push(node);
            }
        }

        let health_of_nodes = self.health_client.nodes().await?;

        let change = subnet_change_request.optimize(
            optimize.unwrap_or(0),
            &to_be_replaced,
            &health_of_nodes,
            self.cordoned_features_fetcher.fetch().await?,
            all_nodes,
        )?;

        for n in change.removed().iter().filter(|n| !node_ids_unhealthy.contains(&n.principal)) {
            motivations.push(format!(
                "replacing {} as per user request{}",
                n.id_short(),
                match motivation {
                    Some(ref m) => format!(": {}", m),
                    None => "".to_string(),
                }
            ));
        }

        let motivation = format!(
                "\n{}\n\nNOTE: The information below is provided for your convenience. Please independently verify the decentralization changes rather than relying solely on this summary.\nCode for calculating replacements is at https://github.com/dfinity/dre/blob/79066127f58c852eaf4adda11610e815a426878c/rs/decentralization/src/network.rs#L912",
                motivations.iter().map(|s| format!(" - {}", s)).collect::<Vec<String>>().join("\n")
            );

        let change = SubnetChangeResponse::new(&change, &health_of_nodes, Some(motivation));

        Ok(change)
    }

    pub async fn subnet_resize(
        &self,
        request: ic_management_types::requests::SubnetResizeRequest,
        proposal_motivation: String,
        health_of_nodes: &IndexMap<PrincipalId, HealthStatus>,
    ) -> anyhow::Result<SubnetChangeResponse> {
        let registry = self.registry_instance.clone();
        let all_nodes = registry.nodes().await?.values().cloned().collect_vec();
        let mut motivations = vec![];

        let change = registry
            .modify_subnet_nodes(SubnetQueryBy::SubnetId(request.subnet))
            .await?
            .excluding_from_available(request.exclude.clone().unwrap_or_default())
            .including_from_available(request.only.clone().unwrap_or_default())
            .including_from_available(request.include.clone().unwrap_or_default())
            .resize(
                request.add,
                request.remove,
                0,
                health_of_nodes,
                self.cordoned_features_fetcher.fetch().await?,
                &all_nodes,
            )?;

        for n in change.removed().iter() {
            motivations.push(format!("removing node {} for subnet resize", n.principal));
        }

        for n in change.added().iter() {
            motivations.push(format!("adding node {} for subnet resize", n.principal));
        }

        let motivation = format!(
                "{}\n{}\n\nNOTE: The information below is provided for your convenience. Please independently verify the decentralization changes rather than relying solely on this summary.\nCode for calculating replacements is at https://github.com/dfinity/dre/blob/79066127f58c852eaf4adda11610e815a426878c/rs/decentralization/src/network.rs#L912",
                proposal_motivation,
                motivations.iter().map(|s| format!(" - {}", s)).collect::<Vec<String>>().join("\n")
            );

        let change = SubnetChangeResponse::new(&change, health_of_nodes, Some(motivation));

        Ok(change)
    }
}
