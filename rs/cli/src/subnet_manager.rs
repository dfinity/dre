use core::fmt;
use std::collections::HashSet;
use std::rc::Rc;

use anyhow::anyhow;
use anyhow::Ok;
use decentralization::{
    network::{DecentralizedSubnet, Node as DecentralizedNode, NodesConverter, SubnetQueryBy, TopologyManager},
    SubnetChangeResponse,
};
use ic_management_backend::health::{self, HealthStatusQuerier};
use ic_management_backend::lazy_registry::LazyRegistry;
use ic_management_types::MinNakamotoCoefficients;
use ic_management_types::Network;
use ic_types::PrincipalId;
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
    registry_instance: Rc<LazyRegistry>,
    network: Network,
}

impl SubnetManager {
    pub fn new(registry_instance: Rc<LazyRegistry>, network: Network) -> Self {
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

    async fn unhealthy_nodes(&self, subnet: DecentralizedSubnet) -> anyhow::Result<Vec<(DecentralizedNode, ic_management_types::Status)>> {
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
                        Some((n, health.clone()))
                    }
                }
                None => {
                    warn!("Node {} has no known health, assuming unhealthy", n.id);
                    Some((n, ic_management_types::Status::Unknown))
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
    /// Principals.
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
        min_nakamoto_coefficients: Option<MinNakamotoCoefficients>,
    ) -> anyhow::Result<SubnetChangeResponse> {
        let subnet_query_by = self.get_subnet_query_by(self.target()?).await?;
        let mut motivations: Vec<String> = if let Some(motivation) = motivation { vec![motivation] } else { vec![] };
        let mut to_be_replaced: Vec<(DecentralizedNode, String)> = if let SubnetQueryBy::NodeList(nodes) = &subnet_query_by {
            nodes.into_iter().map(|n| (n.clone(), "as per user request".to_string())).collect()
        } else {
            vec![]
        };

        let subnet_change_request = self
            .registry_instance
            .modify_subnet_nodes(subnet_query_by.clone())
            .await?
            .excluding_from_available(exclude.clone().unwrap_or_default())
            .including_from_available(only.clone())
            .including_from_available(include.clone().unwrap_or_default())
            .with_min_nakamoto_coefficients(min_nakamoto_coefficients.clone());

        let mut node_ids_unhealthy = HashSet::new();
        if heal {
            let subnet_unhealthy = self.unhealthy_nodes(subnet_change_request.subnet()).await?;
            let subnet_unhealthy_without_included = subnet_unhealthy
                .into_iter()
                .filter(|(n, _)| !include.as_ref().unwrap_or(&vec![]).contains(&n.id))
                .map(|(n, s)| (n, s.to_string().to_lowercase()))
                .collect::<Vec<_>>();

            for (n, reason) in subnet_unhealthy_without_included.iter() {
                motivations.push(format!("replacing {reason} node {}", n.id));
                node_ids_unhealthy.insert(n.id);
            }

            to_be_replaced.extend(subnet_unhealthy_without_included);
        }

        let change = subnet_change_request.optimize(optimize.unwrap_or(0), &to_be_replaced)?;

        for (n, _) in change.removed().iter().filter(|(n, _)| !node_ids_unhealthy.contains(&n.id)) {
            motivations.push(format!("replacing {} to optimize network topology", n.id));
        }

        let motivation = format!(
                "\n{}\n\nNOTE: The information below is provided for your convenience. Please independently verify the decentralization changes rather than relying solely on this summary.\nCode for calculating replacements is at https://github.com/dfinity/dre/blob/79066127f58c852eaf4adda11610e815a426878c/rs/decentralization/src/network.rs#L912\n\n```\n{}\n```\n",
                motivations.iter().map(|s| format!(" - {}", s)).collect::<Vec<String>>().join("\n"),
                change
            );

        let change = SubnetChangeResponse::from(&change).with_motivation(motivation);

        Ok(change)
    }
}
