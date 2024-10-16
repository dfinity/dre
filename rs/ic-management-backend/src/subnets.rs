use decentralization::{
    network::{DecentralizedSubnet, SubnetChange},
    SubnetChangeResponse,
};
use ic_base_types::PrincipalId;
use ic_management_types::{Node, TopologyChangeProposal};
use indexmap::IndexMap;

pub fn get_proposed_subnet_changes(
    all_nodes: &IndexMap<PrincipalId, Node>,
    subnet: &ic_management_types::Subnet,
) -> Result<SubnetChangeResponse, anyhow::Error> {
    if let Some(proposal) = &subnet.proposal {
        let proposal: &TopologyChangeProposal = proposal;
        let subnet_nodes: Vec<_> = subnet.nodes.iter().map(decentralization::network::Node::from).collect();
        let penalties_after_change = DecentralizedSubnet::check_business_rules_for_subnet_with_nodes(&subnet.principal, &subnet_nodes)
            .expect("Business rules check should succeed")
            .0;
        let change = SubnetChange {
            id: subnet.principal,
            old_nodes: subnet_nodes.clone(),
            new_nodes: subnet_nodes,
            removed_nodes_desc: vec![],
            added_nodes_desc: vec![],
            penalties_after_change,
            comment: None,
            run_log: vec![],
        }
        .with_nodes(
            proposal
                .node_ids_added
                .iter()
                .map(|p| {
                    (
                        decentralization::network::Node::from(all_nodes.get(p).unwrap()),
                        "added in proposal".to_string(),
                    )
                })
                .collect::<Vec<_>>(),
        )
        .without_nodes(
            proposal
                .node_ids_removed
                .iter()
                .map(|p| {
                    (
                        decentralization::network::Node::from(all_nodes.get(p).unwrap()),
                        "removed in proposal".to_string(),
                    )
                })
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
        assert_eq!(change.added_with_desc.iter().map(|a| a.0).collect::<Vec<_>>(), node_ids_added);
        assert_eq!(change.removed_with_desc, vec![]);
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
        assert_eq!(change.added_with_desc.iter().map(|a| a.0).collect::<Vec<_>>(), node_ids_added);
        assert_eq!(change.removed_with_desc.iter().map(|x| x.0).collect::<Vec<_>>(), node_ids_removed);
    }

    fn gen_test_nodes(subnet_id: PrincipalId, num_nodes: u64, start_at_number: u64) -> IndexMap<PrincipalId, Node> {
        let mut nodes = IndexMap::new();
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
                        area: format!("datacenter-area-{}", i),
                        country: format!("datacenter-country-{}", i),
                        continent: format!("datacenter-continent-{}", i),
                        latitude: None,
                        longitude: None,
                    }),
                    ..Default::default()
                },
                subnet_id: Some(subnet_id),
                hostos_release: None,
                ip_addr: Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1),
                hostname: None,
                dfinity_owned: None,
                proposal: None,
                duplicates: None,
                label: None,
                hostos_version: "".to_string(),
                is_api_boundary_node: false,
                chip_id: None,
                public_ipv4_config: None,
            };
            nodes.insert(node.principal, node);
        }
        nodes
    }
}
