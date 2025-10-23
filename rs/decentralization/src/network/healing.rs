use super::*;
use crate::{
    SubnetChangeResponse,
    subnets::{subnets_with_business_rules_violations, unhealthy_with_nodes},
};
use log::{info, warn};
use std::{cmp::Ordering, collections::BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkHealSubnet {
    pub name: String,
    pub decentralized_subnet: DecentralizedSubnet,
    pub unhealthy_nodes: Vec<Node>,
    pub cordoned_nodes: Vec<(Node, String)>,
}

impl NetworkHealSubnet {
    const IMPORTANT_SUBNETS: &'static [&'static str] = &["NNS", "SNS", "Bitcoin", "Internet Identity", "tECDSA signing"];
}

impl PartialOrd for NetworkHealSubnet {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NetworkHealSubnet {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_is_important = NetworkHealSubnet::IMPORTANT_SUBNETS.contains(&self.name.as_str());
        let other_is_important = NetworkHealSubnet::IMPORTANT_SUBNETS.contains(&other.name.as_str());

        match (self_is_important, other_is_important) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            _ => self.decentralized_subnet.nodes.len().cmp(&other.decentralized_subnet.nodes.len()),
        }
    }
}

pub struct NetworkHealRequest {
    pub subnets: IndexMap<PrincipalId, ic_management_types::Subnet>,
}

impl NetworkHealRequest {
    pub fn new(subnets: IndexMap<PrincipalId, ic_management_types::Subnet>) -> Self {
        Self { subnets }
    }

    pub async fn fix_and_optimize(
        &self,
        mut available_nodes: Vec<Node>,
        health_of_nodes: &IndexMap<PrincipalId, HealthStatus>,
        cordoned_features: Vec<CordonedFeature>,
        all_nodes: &[Node],
        optimize_for_business_rules_compliance: bool,
        remove_cordoned_nodes: bool,
        heal: bool,
    ) -> Result<Vec<SubnetChangeResponse>, NetworkError> {
        let mut subnets_changed = Vec::new();
        let mut subnets_to_fix: IndexMap<PrincipalId, NetworkHealSubnet> = unhealthy_with_nodes(&self.subnets, health_of_nodes)
            .iter()
            .filter_map(|(subnet_id, unhealthy_nodes)| {
                self.subnets.get(subnet_id).map(|unhealthy_subnet| {
                    (
                        *subnet_id,
                        NetworkHealSubnet {
                            name: unhealthy_subnet.metadata.name.clone(),
                            decentralized_subnet: DecentralizedSubnet::from(unhealthy_subnet),
                            unhealthy_nodes: unhealthy_nodes.clone(),
                            cordoned_nodes: vec![],
                        },
                    )
                })
            })
            .sorted_by(|a, b| b.1.cmp(&a.1))
            .collect();

        if remove_cordoned_nodes {
            for (subnet_id, subnet) in self
                .subnets
                .iter()
                .filter_map(|(subnet_id, subnet)| {
                    let cordoned_nodes: Vec<(Node, String)> = subnet
                        .nodes
                        .iter()
                        .filter_map(|node| {
                            cordoned_features.iter().find_map(|feature| {
                                if node.get_feature(&feature.feature) == Some(feature.value.clone()) {
                                    Some((node.clone(), feature.explanation.clone().unwrap_or_default()))
                                } else {
                                    None
                                }
                            })
                        })
                        .collect();
                    if !cordoned_nodes.is_empty() {
                        Some((
                            *subnet_id,
                            NetworkHealSubnet {
                                name: subnet.metadata.name.clone(),
                                decentralized_subnet: DecentralizedSubnet::from(subnet.clone()),
                                unhealthy_nodes: vec![],
                                cordoned_nodes,
                            },
                        ))
                    } else {
                        None
                    }
                })
                .sorted_by(|a, b| b.1.cmp(&a.1))
            {
                if let Some(existing_subnet) = subnets_to_fix.get_mut(&subnet_id) {
                    existing_subnet.cordoned_nodes.extend(subnet.cordoned_nodes.clone());
                } else {
                    subnets_to_fix.insert(subnet_id, subnet);
                }
            }
        }

        let mut optimized_subnets = BTreeSet::new();
        if optimize_for_business_rules_compliance {
            for subnet in subnets_with_business_rules_violations(&self.subnets.values().cloned().collect::<Vec<_>>()) {
                optimized_subnets.insert(subnet.principal);
                let network_heal_subnet = NetworkHealSubnet {
                    name: subnet.metadata.name.clone(),
                    decentralized_subnet: DecentralizedSubnet::from(subnet),
                    unhealthy_nodes: vec![],
                    cordoned_nodes: vec![],
                };

                if !subnets_to_fix.contains_key(&network_heal_subnet.decentralized_subnet.id) {
                    subnets_to_fix.insert(network_heal_subnet.decentralized_subnet.id, network_heal_subnet);
                }
            }
        }

        // If the healing is not requested, remove the subnets that are JUST healed.
        if !heal {
            subnets_to_fix.retain(|p, val| !val.cordoned_nodes.is_empty() || optimized_subnets.contains(p));
        }

        if subnets_to_fix.is_empty() {
            info!("Nothing to do! All subnets are healthy and compliant with the topology checks.")
        }

        // Re-sort all subnets together to maintain priority ordering and fix the most important subnets first.
        subnets_to_fix = subnets_to_fix.into_iter().sorted_by(|a, b| b.1.cmp(&a.1)).collect();

        for (_subnet_id, subnet) in subnets_to_fix.iter() {
            // If more than 1/3 nodes do not have the latest subnet state, subnet will stall.
            // From those 1/2 are added and 1/2 removed -> nodes_in_subnet/3 * 1/2 = nodes_in_subnet/6
            let max_replaceable_nodes = subnet.decentralized_subnet.nodes.len() / 6;
            let nodes_to_replace: IndexSet<_> = subnet
                .unhealthy_nodes
                .clone()
                .into_iter()
                .map(|node| (node, "unhealthy".to_string()))
                .chain(subnet.cordoned_nodes.clone().into_iter())
                .collect();

            let nodes_to_replace = if nodes_to_replace.len() > max_replaceable_nodes {
                let nodes_to_replace = nodes_to_replace.into_iter().take(max_replaceable_nodes).collect_vec();
                warn!(
                    "Subnet {}: replacing {} of {} unhealthy nodes: {:?}",
                    subnet.decentralized_subnet.id,
                    max_replaceable_nodes,
                    nodes_to_replace.len(),
                    nodes_to_replace.iter().map(|(node, _)| node.principal).collect_vec()
                );
                nodes_to_replace
            } else {
                info!(
                    "Subnet {}: replacing {} nodes; unhealthy {:?}, optimizing {:?}. Max safely replaceable nodes based on subnet size: {}",
                    subnet.decentralized_subnet.id,
                    nodes_to_replace.len(),
                    subnet
                        .unhealthy_nodes
                        .iter()
                        .map(|node| node.principal.to_string().split('-').next().unwrap().to_string())
                        .collect_vec(),
                    subnet
                        .cordoned_nodes
                        .iter()
                        .map(|(node, _)| node.principal.to_string().split('-').next().unwrap().to_string())
                        .collect_vec(),
                    max_replaceable_nodes,
                );
                nodes_to_replace.into_iter().collect_vec()
            };

            let optimize_limit = max_replaceable_nodes - nodes_to_replace.len();
            let change_req = SubnetChangeRequest {
                subnet: subnet.decentralized_subnet.clone(),
                available_nodes: available_nodes.clone(),
                ..Default::default()
            };

            // Try to replace from 0 to optimize_limit nodes to optimize the network,
            // and choose the replacement of the fewest nodes that gives the most decentralization benefit.
            let changes = (0..=optimize_limit)
                .filter_map(|num_nodes_to_optimize| {
                    change_req
                        .clone()
                        .optimize(
                            num_nodes_to_optimize,
                            &nodes_to_replace.iter().map(|(node, _)| node.clone()).collect_vec(),
                            health_of_nodes,
                            cordoned_features.clone(),
                            all_nodes,
                        )
                        .map_err(|e| warn!("{}", e))
                        .ok()
                })
                .map(|change| {
                    SubnetChangeResponse::new(
                        &change,
                        health_of_nodes,
                        Some("Replacing nodes to optimize network topology and heal unhealthy nodes".to_string()),
                    )
                })
                .collect::<Vec<_>>();

            if changes.is_empty() {
                warn!("No suitable changes found for subnet {}", subnet.decentralized_subnet.id);
                continue;
            }

            for change in &changes {
                info!(
                    "Replacing {} nodes in subnet {} results in subnet topology penalty {} and Nakamoto coefficient: {}\n",
                    change.node_ids_removed.len(),
                    subnet.decentralized_subnet.id,
                    change.penalties_after_change.0,
                    change.score_after
                );
            }

            // There is already a check above that "changes" isn't empty
            let penalty_original = changes[0].penalties_before_change.0;
            // Some community members have expressed concern about the subnet-topology (business rules) penalty.
            // https://forum.dfinity.org/t/subnet-management-tdb26-nns/33663/26 and a few comments below.
            // As a compromise, we will choose the change that has the lowest business-rules penalty,
            // or if there is no improvement in the business-rules penalty, we will choose the change
            // that replaces the fewest nodes.
            let penalty_optimize_min = changes.iter().map(|change| change.penalties_after_change.0).min().unwrap();
            info!("Min subnet topology penalty: {}", penalty_optimize_min);

            // Include only solutions with the minimal penalty
            let best_changes = changes
                .iter()
                .filter(|change| change.penalties_after_change.0 == penalty_optimize_min)
                .collect::<Vec<_>>();

            // Then from those with the minimal penalty, find the ones with the maximum Nakamoto Coefficient (best decentralization)
            let changes_max_score = best_changes
                .iter()
                .max_by_key(|change| &change.score_after)
                .expect("Failed to find a replacement with the highest Nakamoto coefficient");
            info!("Best Nakamoto coefficient after the change: {}", changes_max_score.score_after);

            // A solution (subnet membership) gets a penalty based on how far it is from the optimal topology.
            // Lowering the penalty means getting closer to the solution that satisfies all rules of the optimal topology.
            let is_solution_penalty_improving = penalty_optimize_min < penalty_original;

            let all_optimizations_desc = changes
                .iter()
                .enumerate()
                .skip(1)
                .map(|(num_opt, change)| {
                    let previous_index = num_opt.saturating_sub(1);
                    let penalty_before = changes[previous_index].penalties_after_change.0;
                    let penalty_after = change.penalties_after_change.0;
                    format!(
                        "- {} additional node{} would result in: {}{}",
                        num_opt,
                        if num_opt > 1 { "s" } else { "" },
                        change.score_after.describe_difference_from(&changes[previous_index].score_after).1,
                        if penalty_after > 0 || penalty_before != penalty_after {
                            format!(
                                " and subnet topology penalty before {} => {} after the change",
                                penalty_before, penalty_after
                            )
                        } else {
                            "".to_string()
                        },
                    )
                })
                .collect::<Vec<_>>();

            let change = if is_solution_penalty_improving {
                best_changes
                    .iter()
                    .find(|change| change.score_after == changes_max_score.score_after)
                    .cloned()
                    .expect("Failed to find the expected replacement with the maximum Nakamoto Coefficient")
            } else {
                info!("No reduction in the subnet topology penalty, choosing the first change");
                best_changes[0]
            };

            if change.node_ids_removed.is_empty() {
                warn!("No suitable changes found for subnet {}", subnet.decentralized_subnet.id);
                continue;
            }

            info!(
                "Replacing {} nodes in subnet {} gives Nakamoto coefficient: {}\n",
                change.node_ids_removed.len(),
                subnet.decentralized_subnet.id,
                change.score_after
            );

            let num_opt = change.node_ids_removed.len() - nodes_to_replace.len();
            let reason_additional_optimizations = if num_opt == 0 {
                format!(
                    "

Calculated potential impact on subnet decentralization if replacing:

{}

Based on the calculated potential impact, not replacing additional nodes to improve optimization.
",
                    all_optimizations_desc.join("\n")
                )
            } else {
                format!("

Calculated potential impact on subnet decentralization if replacing:

{}

Based on the calculated potential impact, replacing {} additional nodes to improve optimization

Note: the heuristic for node replacement relies not only on the Nakamoto coefficient but also on other factors that iteratively optimize network topology.
Due to this, Nakamoto coefficients may not directly increase in every node replacement proposal.
Code for comparing decentralization of two candidate subnet topologies is at:
https://github.com/dfinity/dre/blob/79066127f58c852eaf4adda11610e815a426878c/rs/decentralization/src/nakamoto/mod.rs#L342
",
                    all_optimizations_desc.join("\n"),
                    num_opt
                )
            };

            let mut motivations: Vec<String> = Vec::new();

            let unhealthy_nodes_ids = subnet.unhealthy_nodes.iter().map(|node| node.principal).collect::<HashSet<_>>();
            let cordoned_nodes_ids = subnet.cordoned_nodes.iter().map(|(node, _)| node.principal).collect::<HashSet<_>>();

            for (node, desc) in nodes_to_replace.iter() {
                let node_id_short = node
                    .principal
                    .to_string()
                    .split_once('-')
                    .expect("subnet id is expected to have a -")
                    .0
                    .to_string();
                if unhealthy_nodes_ids.contains(&node.principal) {
                    motivations.push(format!(
                        "replacing {} node {}",
                        health_of_nodes
                            .get(&node.principal)
                            .map(|s| s.to_string().to_lowercase())
                            .unwrap_or("unhealthy".to_string()),
                        node_id_short
                    ));
                } else if cordoned_nodes_ids.contains(&node.principal) {
                    motivations.push(format!("replacing cordoned node {} ({})", node_id_short, desc));
                } else if is_solution_penalty_improving {
                    motivations.push(format!("replacing node {} to reduce subnet topology penalty", node_id_short));
                } else {
                    motivations.push(format!("replacing node {} to optimize network topology", node_id_short));
                };
            }

            available_nodes.retain(|node| !change.node_ids_added.contains(&node.principal));

            let motivation = format!(
                "\n{}{}\nNote: the information below is provided for your convenience. Please independently verify the decentralization changes rather than relying solely on this summary.\nHere is [an explaination of how decentralization is currently calculated](https://dfinity.github.io/dre/decentralization.html), \nand there are also [instructions for performing what-if analysis](https://dfinity.github.io/dre/subnet-decentralization-whatif.html) if you are wondering if another node would have improved decentralization more.\n\n",
                motivations.iter().map(|s| format!(" - {}", s)).collect::<Vec<String>>().join("\n"),
                reason_additional_optimizations
            );
            subnets_changed.push(change.clone().with_motivation(motivation));
        }

        Ok(subnets_changed)
    }
}
