use std::collections::BTreeMap;

use crate::health;
use crate::health::HealthStatusQuerier;
use crate::registry::RegistryState;
use decentralization::{network::SubnetChange, SubnetChangeResponse};
use ic_base_types::PrincipalId;
use ic_management_types::{Node, TopologyChangeProposal};
use log::{info, warn};
use tokio::sync::RwLockReadGuard;

pub async fn get_unhealthy(
    registry: &RwLockReadGuard<'_, RegistryState>,
) -> anyhow::Result<BTreeMap<PrincipalId, Vec<decentralization::network::Node>>> {
    let health_client = health::HealthClient::new(registry.network());
    let healths = health_client.nodes().await?;

    let unhealthy_subnets = registry
        .subnets()
        .into_iter()
        .filter_map(|(_, subnet)| {
            let unhealthy = subnet
                .nodes
                .into_iter()
                .filter_map(|n| match healths.get(&n.principal) {
                    Some(health) => {
                        if *health == ic_management_types::Status::Healthy {
                            None
                        } else {
                            info!("Node {} is {:?}", n.principal, health);
                            Some(n)
                        }
                    }
                    None => {
                        warn!("Node {} has no known health, assuming unhealthy", n.principal);
                        Some(n)
                    }
                })
                .map(|n| decentralization::network::Node::from(&n))
                .collect::<Vec<_>>();

            if !unhealthy.is_empty() {
                Some((subnet.principal, unhealthy))
            } else {
                None
            }
        })
        .collect::<BTreeMap<_, _>>();

    Ok(unhealthy_subnets)
}

pub fn get_proposed_subnet_changes(
    all_nodes: &BTreeMap<PrincipalId, Node>,
    subnet: &ic_management_types::Subnet,
) -> Result<SubnetChangeResponse, anyhow::Error> {
    if let Some(proposal) = &subnet.proposal {
        let proposal: &TopologyChangeProposal = proposal;
        let change = SubnetChange {
            id: subnet.principal,
            old_nodes: subnet.nodes.iter().map(decentralization::network::Node::from).collect(),
            new_nodes: subnet.nodes.iter().map(decentralization::network::Node::from).collect(),
            min_nakamoto_coefficients: None,
            comment: None,
            run_log: vec![],
        }
        .with_nodes(
            proposal
                .node_ids_added
                .iter()
                .map(|p| decentralization::network::Node::from(all_nodes.get(p).unwrap()))
                .collect::<Vec<_>>(),
        )
        .without_nodes(
            proposal
                .node_ids_removed
                .iter()
                .map(|p| decentralization::network::Node::from(all_nodes.get(p).unwrap()))
                .collect::<Vec<_>>(),
        );
        let mut response = SubnetChangeResponse::from(&change);
        response.proposal_id = Some(proposal.id);
        Ok(response)
    } else {
        Err(anyhow::format_err!(
            "subnet {} does not have open membership change proposals",
            subnet.principal
        ))
    }
}

// Adding some tests to the above function
#[cfg(test)]
mod tests {
    use std::net::Ipv6Addr;

    use ic_management_types::{Datacenter, DatacenterOwner, Operator, Provider};

    use super::*;

    #[test]
    fn test_subnet_changes_for_empty() {
        // Create some test nodes
        let subnet_id = PrincipalId::new_subnet_test_id(0);
        let all_nodes = gen_test_nodes(subnet_id, 50, 0);
        let subnet = ic_management_types::Subnet {
            principal: subnet_id,
            nodes: all_nodes.values().take(13).cloned().collect(),
            ..Default::default()
        };
        let err = get_proposed_subnet_changes(&all_nodes, &subnet).unwrap_err().to_string();
        assert_eq!(err, "subnet fscpm-uiaaa-aaaaa-aaaap-yai does not have open membership change proposals");
    }

    #[test]
    fn test_subnet_changes_for_1_node() {
        let subnet_id = PrincipalId::new_subnet_test_id(0);
        let all_nodes = gen_test_nodes(subnet_id, 50, 0);
        let subnet_size = 13;
        let node_ids_added: Vec<PrincipalId> = all_nodes.keys().skip(subnet_size).take(1).cloned().collect();
        let proposal_replace = TopologyChangeProposal {
            node_ids_added: node_ids_added.clone(),
            node_ids_removed: vec![],
            subnet_id: Some(subnet_id),
            id: 12345,
        };
        let subnet = ic_management_types::Subnet {
            principal: subnet_id,
            nodes: all_nodes.values().take(subnet_size).cloned().collect(),
            proposal: Some(proposal_replace),
            ..Default::default()
        };
        let change = get_proposed_subnet_changes(&all_nodes, &subnet).unwrap();
        assert_eq!(change.added, node_ids_added);
        assert_eq!(change.removed, vec![]);
    }

    #[test]
    fn test_subnet_changes_for_multiple_add_remove_nodes() {
        let subnet_id = PrincipalId::new_subnet_test_id(0);
        let all_nodes = gen_test_nodes(subnet_id, 50, 0);
        let subnet_size = 13;
        let node_ids_added: Vec<PrincipalId> = all_nodes.keys().skip(subnet_size).take(3).cloned().collect();
        let node_ids_removed: Vec<PrincipalId> = all_nodes.keys().take(2).cloned().collect();
        let proposal_replace = TopologyChangeProposal {
            node_ids_added: node_ids_added.clone(),
            node_ids_removed: node_ids_removed.clone(),
            subnet_id: Some(subnet_id),
            id: 12345,
        };
        let subnet = ic_management_types::Subnet {
            principal: subnet_id,
            nodes: all_nodes.values().take(subnet_size).cloned().collect(),
            proposal: Some(proposal_replace),
            ..Default::default()
        };
        let change = get_proposed_subnet_changes(&all_nodes, &subnet).unwrap();
        assert_eq!(change.added, node_ids_added);
        assert_eq!(change.removed, node_ids_removed);
    }

    fn gen_test_nodes(subnet_id: PrincipalId, num_nodes: u64, start_at_number: u64) -> BTreeMap<PrincipalId, Node> {
        let mut nodes = BTreeMap::new();
        for i in start_at_number..start_at_number + num_nodes {
            let node = Node {
                principal: PrincipalId::new_node_test_id(i),
                operator: Operator {
                    principal: PrincipalId::new_user_test_id(i),
                    provider: Provider {
                        principal: PrincipalId::new_user_test_id(i),
                        name: Some(format!("provider-{}", i)),
                        ..Default::default()
                    },
                    datacenter: Some(Datacenter {
                        name: format!("datacenter-{}", i),
                        owner: DatacenterOwner {
                            name: format!("datacenter-owner-{}", i),
                        },
                        city: format!("datacenter-city-{}", i),
                        country: format!("datacenter-country-{}", i),
                        continent: format!("datacenter-continent-{}", i),
                        latitude: None,
                        longitude: None,
                    }),
                    ..Default::default()
                },
                subnet_id: Some(subnet_id),
                hostos_release: None,
                decentralized: true,
                ip_addr: Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1),
                hostname: None,
                dfinity_owned: None,
                proposal: None,
                duplicates: None,
                label: None,
                hostos_version: "".to_string(),
                is_api_boundary_node: false,
            };
            nodes.insert(node.principal, node);
        }
        nodes
    }
}
