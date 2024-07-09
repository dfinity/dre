use core::fmt;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::anyhow;
use anyhow::Ok;
use decentralization::{
    network::{DecentralizedSubnet, Node as DecentralizedNode, NodesConverter, SubnetQueryBy, TopologyManager},
    SubnetChangeResponse,
};
use ic_management_backend::lazy_registry::LazyRegistry;
use ic_management_backend::{
    health::{self, HealthStatusQuerier},
    registry::RegistryState,
};
use ic_management_types::MinNakamotoCoefficients;
use ic_management_types::Network;
use ic_types::PrincipalId;
use itertools::Itertools;
use log::{info, warn};

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
    registry_instance: Arc<LazyRegistry>,
    network: Network,
}

impl SubnetManager {
    pub fn new(registry_instance: Arc<LazyRegistry>, network: Network) -> Self {
        Self {
            subnet_target: None,
            registry_instance,
            network,
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

    async fn unhealthy_nodes(&self, subnet: DecentralizedSubnet) -> anyhow::Result<Vec<DecentralizedNode>> {
        let health_client = health::HealthClient::new(self.network.clone());
        let subnet_health = health_client.subnet(subnet.id).await?;

        let unhealthy = subnet
            .nodes
            .into_iter()
            .filter_map(|n| match subnet_health.get(&n.id) {
                Some(health) => {
                    if *health == ic_management_types::Status::Healthy {
                        None
                    } else {
                        info!("Node {} is {:?}", n.id, health);
                        Some(n)
                    }
                }
                None => {
                    warn!("Node {} has no known health, assuming unhealthy", n.id);
                    Some(n)
                }
            })
            .collect::<Vec<_>>();
        Ok(unhealthy)
    }

    async fn get_subnet_query_by(&self, target: SubnetTarget) -> anyhow::Result<SubnetQueryBy> {
        let converted = match target {
            SubnetTarget::FromId(id) => SubnetQueryBy::SubnetId(id),
            SubnetTarget::FromNodesIds(nodes) => {
                let nodes = self.registry_instance.get_nodes(&nodes)?;
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
    /// Principals.
    ///
    /// All nodes in the request must belong to exactly one subnet.
    pub async fn membership_replace(
        &self,
        heal: bool,
        motivation: String,
        optimize: Option<usize>,
        exclude: Option<Vec<String>>,
        only: Vec<String>,
        include: Option<Vec<PrincipalId>>,
        min_nakamoto_coefficients: Option<MinNakamotoCoefficients>,
    ) -> anyhow::Result<SubnetChangeResponse> {
        let subnet_query_by = self.get_subnet_query_by(self.target()?).await?;
        let mut motivations: Vec<String> = vec![motivation];
        let mut to_be_replaced: Vec<DecentralizedNode> = vec![];

        let subnet_change_request = self
            .registry_instance
            .modify_subnet_nodes(subnet_query_by.clone())
            .await?
            .excluding_from_available(exclude.clone().unwrap_or_default())
            .including_from_available(only.clone())
            .including_from_available(include.clone().unwrap_or_default())
            .with_min_nakamoto_coefficients(min_nakamoto_coefficients.clone());

        if heal {
            let subnet_unhealthy = self.unhealthy_nodes(subnet_change_request.subnet()).await?;
            let subnet_unhealthy_without_included = subnet_unhealthy
                .into_iter()
                .filter(|n| !include.as_ref().unwrap_or(&vec![]).contains(&n.id))
                .collect::<Vec<_>>();

            to_be_replaced.extend(subnet_unhealthy_without_included);

            let without_specified = to_be_replaced
                .iter()
                .filter(|n| match &subnet_query_by {
                    SubnetQueryBy::NodeList(nodes) => !nodes.contains(n),
                    _ => true,
                })
                .collect_vec();

            if !without_specified.is_empty() {
                let num_unhealthy = without_specified.len();
                let replace_target = if num_unhealthy == 1 { "node" } else { "nodes" };
                motivations.push(format!("replacing {num_unhealthy} unhealthy {replace_target}"));
            }
        }

        let change = subnet_change_request.optimize(optimize.unwrap_or(0), &to_be_replaced)?;
        let num_optimized = change.removed().len() - to_be_replaced.len();

        if num_optimized > 0 {
            let replace_target = if num_optimized == 1 { "node" } else { "nodes" };
            motivations.push(format!("replacing {num_optimized} {replace_target} to improve subnet decentralization"));
        }

        let change = SubnetChangeResponse::from(&change).with_motivation(motivations.join("; "));

        Ok(change)
    }
}
