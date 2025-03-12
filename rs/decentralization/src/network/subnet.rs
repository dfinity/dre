use super::*;
use crate::nakamoto::NakamotoScore;
use crate::provider_clusters::get_linked_providers;
use log::{debug, info};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecentralizedSubnet {
    pub id: PrincipalId,
    pub nodes: Vec<Node>,
    pub added_nodes: Vec<Node>,
    pub removed_nodes: Vec<Node>,
    pub comment: Option<String>,
    pub run_log: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ReplacementCandidate {
    pub(crate) node: Node,
    score: NakamotoScore,
    penalty: usize,
    business_rules_log: Vec<String>,
}

impl DecentralizedSubnet {
    pub fn new_with_subnet_id_and_nodes(subnet_id: PrincipalId, nodes: Vec<Node>) -> Self {
        Self {
            id: subnet_id,
            nodes,
            added_nodes: vec![],
            removed_nodes: vec![],
            comment: None,
            run_log: vec![],
        }
    }

    pub fn with_subnet_id(self, subnet_id: PrincipalId) -> Self {
        Self { id: subnet_id, ..self }
    }

    /// Return a new instance of a DecentralizedSubnet that does not contain the
    /// provided nodes.
    pub fn without_nodes(&self, nodes_to_remove: &[Node]) -> Result<Self, NetworkError> {
        let mut new_subnet_nodes = self.nodes.clone();
        let mut removed_nodes = self.removed_nodes.clone();
        for node in nodes_to_remove {
            if let Some(index) = new_subnet_nodes.iter().position(|n| n.principal == node.principal) {
                removed_nodes.push(new_subnet_nodes.remove(index));
            } else {
                return Err(NetworkError::NodeNotFound(node.principal));
            }
        }
        let removed_is_empty = removed_nodes.is_empty();
        let removed_node_ids = removed_nodes.iter().map(|n| n.principal).collect::<Vec<_>>();
        if !removed_is_empty {
            assert!(new_subnet_nodes.len() <= self.nodes.len());
        }
        Ok(Self {
            id: self.id,
            nodes: new_subnet_nodes,
            added_nodes: self.added_nodes.clone(),
            removed_nodes,
            comment: self.comment.clone(),
            run_log: {
                if removed_is_empty {
                    self.run_log.clone()
                } else {
                    let mut run_log = self.run_log.clone();
                    run_log.push(format!("Removed nodes from subnet {:?}", removed_node_ids));
                    run_log
                }
            },
        })
    }

    /// Return a new instance of a DecentralizedSubnet that contains the
    /// provided nodes.
    pub fn with_nodes(self, nodes_to_add: &[Node]) -> Self {
        let subnet_nodes_after_adding: Vec<Node> = self.nodes.clone().into_iter().chain(nodes_to_add.to_vec()).collect();
        let added_nodes = [self.added_nodes, nodes_to_add.to_vec()].concat();
        if !nodes_to_add.is_empty() {
            assert!(subnet_nodes_after_adding.len() > self.nodes.len());
        }
        Self {
            id: self.id,
            nodes: subnet_nodes_after_adding,
            added_nodes,
            removed_nodes: self.removed_nodes,
            comment: self.comment,
            run_log: {
                if nodes_to_add.is_empty() {
                    self.run_log
                } else {
                    let mut run_log = self.run_log;
                    run_log.push(format!(
                        "Including user-provided nodes {:?}",
                        nodes_to_add
                            .iter()
                            .map(|n| n.to_string().split_once('-').unwrap_or_default().0.to_string())
                            .collect::<Vec<_>>()
                    ));
                    run_log
                }
            },
        }
    }

    /// Return a list of nodes that are under control of the most dominant
    /// feature value. For instance with the argument NodeProvider, it will
    /// return the nodes that are under control of the most dominant
    /// NodeProvider.
    pub fn nodes_under_control_of_dominant_actor(&self, node_feature: &NodeFeature) -> Vec<Node> {
        let dominant_feature = self
            .nakamoto_score()
            .feature_value_counts_max(node_feature)
            .map(|(provider, _)| provider)
            .unwrap_or_default();

        self.nodes
            .iter()
            .filter(|n| n.get_feature(node_feature).unwrap() == dominant_feature)
            .cloned()
            .collect()
    }

    /// Check the "business rules" for the current DecentralizedSubnet.
    pub fn check_business_rules(&self) -> anyhow::Result<(usize, Vec<String>)> {
        Self::check_business_rules_for_subnet_with_nodes(&self.id, &self.nodes)
    }

    /// Ensure "business rules" or constraints are met for the subnet id with provided list of nodes.
    /// For instance, there needs to be at least one DFINITY-owned node in each subnet.
    /// For the mainnet NNS there needs to be at least 3 DFINITY-owned nodes.
    pub fn check_business_rules_for_subnet_with_nodes(subnet_id: &PrincipalId, nodes: &[Node]) -> anyhow::Result<(usize, Vec<String>)> {
        let mut checks = Vec::new();
        let mut penalties = 0;
        if nodes.len() <= 1 {
            return Ok((1, checks));
        }

        let nakamoto_scores = Self::_calc_nakamoto_score(nodes);
        let subnet_id_str = subnet_id.to_string();
        let is_european_subnet = &subnet_id_str == "bkfrj-6k62g-dycql-7h53p-atvkj-zg4to-gaogh-netha-ptybj-ntsgw-rqe";
        let is_nns_subnet = &subnet_id_str == "tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe";

        let dfinity_owned_nodes_count: usize = nodes.iter().map(|n| n.dfinity_owned.unwrap_or_default() as usize).sum();
        let target_dfinity_owned_nodes_count = if is_nns_subnet { 3 } else { 1 };

        if dfinity_owned_nodes_count != target_dfinity_owned_nodes_count {
            checks.push(format!(
                "Subnet should have {} DFINITY-owned node(s) for subnet recovery, got {}",
                target_dfinity_owned_nodes_count, dfinity_owned_nodes_count
            ));
            penalties += target_dfinity_owned_nodes_count.abs_diff(dfinity_owned_nodes_count) * 1000;
        }

        if subnet_id_str == *"uzr34-akd3s-xrdag-3ql62-ocgoh-ld2ao-tamcv-54e7j-krwgb-2gm4z-oqe"
            || is_nns_subnet
            || subnet_id_str == *"x33ed-h457x-bsgyx-oqxqf-6pzwv-wkhzr-rm2j3-npodi-purzm-n66cg-gae"
        {
            // We keep the backup of the ECDSA key on uzr34, and we don't want a single
            // country to be able to extract that key.
            // The tECDSA key can be extracted with 1/3 of the nodes.
            // We should use the same NC requirements for uzr34 and the upcoming ECDSA
            // subnet, since they'll both hold the same valuable key.
            // Slack discussion: https://dfinity.slack.com/archives/C01DB8MQ5M1/p1668702249558389
            // For different reasons, there is the same requirement for the NNS and the SNS
            // subnet.
            let feature = NodeFeature::Country;
            let controlled_nodes_max = nodes.len() / 3;
            for (country, count) in nakamoto_scores
                .feature_value_counts(&feature)
                .iter()
                .filter(|(_country, count)| *count > controlled_nodes_max)
            {
                let penalty = (count - controlled_nodes_max) * 1000;
                checks.push(format!(
                    "Country {} controls {} of nodes, which is > {} (1/3 - 1) of subnet nodes. Applying penalty of {}.",
                    country, count, controlled_nodes_max, penalty
                ));
                penalties += penalty;
            }
        }

        // As per the adopted target topology
        // https://dashboard.internetcomputer.org/proposal/132136
        let max_nodes_per_np_and_dc = 1;
        let dfinity_np = "bvcsg-3od6r-jnydw-eysln-aql7w-td5zn-ay5m6-sibd2-jzojt-anwag-mqe";
        let dfinity_dcs = nodes
            .iter()
            .filter(|n| n.operator.provider.principal.to_string() == dfinity_np)
            .map(|n| n.operator.datacenter.clone().unwrap_or_default().name)
            .collect::<AHashSet<_>>();
        let dfinity_dc_owners = nodes
            .iter()
            .filter(|n| n.operator.provider.principal.to_string() == dfinity_np)
            .map(|n| n.operator.datacenter.clone().unwrap_or_default().owner.name)
            .collect::<AHashSet<_>>();

        for feature in &[NodeFeature::NodeProvider, NodeFeature::DataCenter, NodeFeature::DataCenterOwner] {
            for (name, count) in nakamoto_scores
                .feature_value_counts(feature)
                .iter()
                .filter(|(_name, count)| *count > max_nodes_per_np_and_dc)
            {
                // DFINITY is allowed to have 3 nodes on the NNS subnet, exempt from the standard “1 node per DC/NP” rule: https://dashboard.internetcomputer.org/proposal/135700
                if is_nns_subnet && *count <= target_dfinity_owned_nodes_count {
                    if feature == &NodeFeature::NodeProvider && name == dfinity_np {
                        continue;
                    }
                    if feature == &NodeFeature::DataCenter && dfinity_dcs.contains(name) {
                        continue;
                    }
                    if feature == &NodeFeature::DataCenterOwner && dfinity_dc_owners.contains(name) {
                        continue;
                    }
                }
                if *count > max_nodes_per_np_and_dc {
                    let penalty = (count - max_nodes_per_np_and_dc) * 10;
                    checks.push(format!(
                        "{} {} controls {} of nodes, which is higher than target of {} for the subnet. Applying penalty of {}.",
                        feature, name, count, max_nodes_per_np_and_dc, penalty
                    ));
                    penalties += penalty;
                }
            }
        }

        // As per the adopted target topology
        // https://dashboard.internetcomputer.org/proposal/132136
        let max_nodes_per_country = match subnet_id_str.as_str() {
            "tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe"
            | "x33ed-h457x-bsgyx-oqxqf-6pzwv-wkhzr-rm2j3-npodi-purzm-n66cg-gae"
            | "pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae"
            | "uzr34-akd3s-xrdag-3ql62-ocgoh-ld2ao-tamcv-54e7j-krwgb-2gm4z-oqe" => 3,
            _ => 2,
        };
        for (name, count) in nakamoto_scores.feature_value_counts(&NodeFeature::Country) {
            if is_european_subnet && !Node::is_country_from_eu(name.as_str()) {
                // European subnet is expected to be controlled by European countries
            } else if count > max_nodes_per_country {
                let penalty = (count - max_nodes_per_country) * 10;
                checks.push(format!(
                    "Country {} controls {} of nodes, which is higher than target of {} for the subnet. Applying penalty of {}.",
                    name, count, max_nodes_per_country, penalty
                ));
                penalties += penalty;
            }
        }

        if is_european_subnet {
            // European subnet should only take European nodes.
            let country_counts = nakamoto_scores.feature_value_counts(&NodeFeature::Country);
            let non_european_nodes_count = country_counts
                .iter()
                .filter_map(|(country, count)| {
                    if Node::is_country_from_eu(country.as_str()) || country.as_str() == "CH" || country.as_str() == "UK" {
                        None
                    } else {
                        Some(*count)
                    }
                })
                .sum::<usize>();
            if non_european_nodes_count > 0 {
                checks.push(format!("European subnet has {} non-European node(s)", non_european_nodes_count));
                penalties += non_european_nodes_count * 1000;
            }
        }

        match nakamoto_scores.score_feature(&NodeFeature::NodeProvider) {
            Some(score) => {
                if score <= 1.0 && nodes.len() > 3 {
                    // We restrict to subnets with >3 nodes to be able to build subnet from scratch
                    checks.push("A single Node Provider can halt the subnet".to_string());
                    penalties += 10000;
                }
            }
            None => return Err(anyhow::anyhow!("Missing the Nakamoto score for the Node Provider")),
        }

        for feature in &NodeFeature::variants() {
            match (nakamoto_scores.score_feature(feature), nakamoto_scores.controlled_nodes(feature)) {
                (Some(score), Some(controlled_nodes)) => {
                    let european_subnet_penalty = is_european_subnet && feature == &NodeFeature::Country;

                    if score == 1.0 && controlled_nodes > nodes.len() * 2 / 3 && !european_subnet_penalty {
                        checks.push(format!(
                            "NodeFeature {} controls {} of nodes, which is > {} (2/3 of all) nodes",
                            feature,
                            controlled_nodes,
                            nodes.len() * 2 / 3
                        ));
                        penalties += (controlled_nodes - nodes.len() * 2 / 3) * 1000;
                    }
                }
                (score, controlled_nodes) => {
                    debug!(
                        "NodeFeature {} does not have valid score {:?} controlled_nodes {:?}",
                        feature.to_string(),
                        &score,
                        &controlled_nodes
                    );
                }
            }
        }

        let mut cluster_counter = AHashMap::new();
        // Count how many nodes in the subnet could be controlled by linked providers
        for provider_id in nodes.iter().map(|n| n.operator.provider.principal).collect_vec() {
            for (pl_name, pl_providers) in get_linked_providers() {
                if pl_providers.contains(&provider_id) {
                    *cluster_counter.entry(pl_name.clone()).or_insert(0) += 1;
                }
            }
        }
        // Apply penalties for participation in clusters
        for (pl_name, count) in cluster_counter {
            if count > 1 {
                checks.push(format!("{} has {} nodes in the subnet", pl_name, count));
                penalties += 10 * (count - 1);
            }
        }

        debug!("Business rules checks succeeded for subnet {}: {:?}", subnet_id.to_string(), checks);
        Ok((penalties, checks))
    }

    fn _calc_nakamoto_score(nodes: &[Node]) -> NakamotoScore {
        NakamotoScore::new_from_nodes(nodes)
    }

    /// Calculate and return the NakamotoScore for the nodes in the subnet
    pub fn nakamoto_score(&self) -> NakamotoScore {
        Self::_calc_nakamoto_score(&self.nodes)
    }

    /// Deterministically choose a result in the list based on the list
    /// of current nodes.  Since the node IDs are unique, we seed a PRNG
    /// with the sorted joined node IDs. We then choose a result
    /// randomly but deterministically using this seed.
    pub(crate) fn choose_one_result(
        best_results: &[ReplacementCandidate],
        current_nodes: &[Node],
        all_nodes: &[Node],
    ) -> Option<ReplacementCandidate> {
        if best_results.is_empty() {
            None
        } else {
            // If any of the best_results nodes are already in the subnet,
            // we should prefer them. This is because we want to keep the
            // same nodes in the subnet if they are already there.
            let current_nodes_set: AHashSet<_> = current_nodes.iter().collect();
            for result in best_results {
                if current_nodes_set.contains(&result.node) {
                    return Some(result.clone());
                }
            }
            // If none of the best_results nodes are already in the subnet,
            // sort the nodes by the absolute number of nodes that the node operator has
            // that are not assigned to subnets and choose the one with the highest number.
            let num_nodes_per_operator = all_nodes.iter().fold(AHashMap::new(), |mut acc: AHashMap<PrincipalId, u32>, n| {
                *acc.entry(n.operator.principal).or_insert(0) += 1;
                acc
            });
            let num_nodes_assigned_to_subnets_per_operator =
                all_nodes
                    .iter()
                    .filter(|n| n.subnet_id.is_some())
                    .fold(AHashMap::new(), |mut acc: AHashMap<PrincipalId, u32>, n| {
                        *acc.entry(n.operator.principal).or_insert(0) += 1;
                        acc
                    });
            let num_nodes_not_assigned_per_operator = num_nodes_per_operator
                .iter()
                .map(|(operator, num_nodes)| {
                    let num_nodes_in_subnet = num_nodes_assigned_to_subnets_per_operator.get(operator).copied().unwrap_or_default();
                    (*operator, *num_nodes as i32 - num_nodes_in_subnet as i32)
                })
                .collect::<AHashMap<PrincipalId, i32>>();
            let best_results = best_results
                .iter()
                .map(|r| {
                    let num_not_assigned = num_nodes_not_assigned_per_operator
                        .get(&r.node.operator.principal)
                        .copied()
                        .unwrap_or_default();
                    let op_nodes = num_nodes_per_operator.get(&r.node.operator.principal).copied().unwrap_or_default() as i32;
                    (num_not_assigned, op_nodes, r)
                })
                // sorted_by_key sorts ascending, so we negate the number of nodes
                // we prefer candidate nodes from operators with:
                //  - the highest number of nodes not assigned to subnets
                //  - highest number of nodes total (to prefer operators with more nodes)
                .sorted_by_key(|(num_not_assigned, op_nodes, _res)| (-num_not_assigned, -op_nodes))
                .collect_vec();
            // filter all the results with the same highest number of nodes not assigned to subnets
            let best_results = best_results
                .iter()
                .take_while(|(num_not_assigned, op_nodes, _res)| *num_not_assigned == best_results[0].0 && *op_nodes == best_results[0].1)
                .map(|(_num_not_assigned, _op_nodes, res)| (*res).clone())
                .collect::<Vec<_>>();

            // We sort the current nodes by alphabetical order on their
            // PrincipalIDs to ensure consistency of the seed with the
            // same machines in the subnet
            let mut id_sorted_current_nodes = current_nodes.to_owned();
            id_sorted_current_nodes.sort_by(|n1, n2| std::cmp::Ord::cmp(&n1.principal.to_string(), &n2.principal.to_string()));
            let seed = rand_seeder::Seeder::from(
                id_sorted_current_nodes
                    .iter()
                    .map(|n| n.principal.to_string())
                    .collect::<Vec<String>>()
                    .join("_"),
            )
            .make_seed();
            let mut rng = rand::rngs::StdRng::from_seed(seed);

            // We sort the best results the same way to ensure that for
            // the same set of machines with the best score, we always
            // get the same one.
            let mut id_sorted_best_results = best_results.to_owned();
            id_sorted_best_results.sort_by(|r1, r2| std::cmp::Ord::cmp(&r1.node.principal.to_string(), &r2.node.principal.to_string()));
            id_sorted_best_results.choose(&mut rng).cloned()
        }
    }

    /// Pick the best result amongst the list of "suitable" candidates.
    fn choose_best_candidate(
        &self,
        candidates: Vec<ReplacementCandidate>,
        run_log: &mut Vec<String>,
        all_nodes: &[Node],
    ) -> Option<ReplacementCandidate> {
        // First, sort the candidates by their Nakamoto Coefficients
        let candidates = candidates
            .into_iter()
            .sorted_by(|a, b| {
                // Prefer nodes with lower penalty. This is for example used to prefer
                // non-DFINITY nodes
                let mut cmp = b.penalty.cmp(&a.penalty);

                if cmp == Ordering::Equal {
                    // Then fallback to comparing the NakamotoScore (custom comparison)
                    debug!("Comparing node {:?} and {:?}", a.node, b.node);
                    cmp = a.score.cmp(&b.score);
                }
                if cmp == Ordering::Less {
                    debug!("Better node is {}", a.node.principal);
                } else {
                    debug!("Better node is {}", b.node.principal);
                }
                cmp
            })
            .collect::<Vec<ReplacementCandidate>>();

        run_log.push("Sorted candidate nodes, with the best candidate at the end:".to_string());
        run_log.push("     <node-id>                                                      <penalty>  <Nakamoto score>".to_string());
        for s in &candidates {
            run_log.push(format!(" -=> {} {} {}", s.node.principal, s.penalty, s.score));
        }

        // Then, pick the candidates with the best (highest) Nakamoto Coefficients.
        // There can be multiple candidates with the same Nakamoto Coefficient.
        let first_best_result = candidates.iter().last();
        let mut best_results = vec![];
        if let Some(result) = first_best_result {
            for candidate in candidates.iter().rev() {
                // To filter the best results, we must take the penalty
                // applied to the subnet as well.  If not, even if two
                // candidates have the same score, we could end up with a
                // higher penalty in the resulting subnet as we choose
                // randomly one of the best candidates in those results We
                // know from the previous sorting that the last element in
                // the array of results will have the lowest penalty and
                // nakamoto score, so we can compare against this one.
                if candidate.score == result.score && candidate.penalty <= result.penalty {
                    best_results.push(candidate.clone())
                } else {
                    break;
                }
            }
        }

        // Given that we have a big pool of unassigned machines, we can
        // randomly but deterministically choose a result amongst the best
        // ones obtained by calculating the new Nakamoto scores. With this
        // big pool of machines, choosing randomly a machine to use or
        // maximizing the decentralization of the remaining available
        // machines will not make a big difference to the final
        // decentralization coefficients.
        //
        // An other approach that was imagined was to maximize the score for
        // the remaining available nodes. However, this approach was too
        // computationally intensive and took too long to compute. Thus, a
        // simpler but good enough method was chosen for choosing a result
        //
        // This approach also has the advantage of not favoring one NP over
        // an other, regardless of the Node PrincipalID
        DecentralizedSubnet::choose_one_result(&best_results, &self.nodes, all_nodes)
    }

    pub(crate) fn _node_to_replacement_candidate(
        &self,
        subnet_nodes: &[Node],
        touched_node: &Node,
        err_log: &mut Vec<String>,
    ) -> Option<ReplacementCandidate> {
        match Self::check_business_rules_for_subnet_with_nodes(&self.id, subnet_nodes) {
            Ok((penalty, business_rules_log)) => {
                let new_score = Self::_calc_nakamoto_score(subnet_nodes);
                Some(ReplacementCandidate {
                    node: touched_node.clone(),
                    score: new_score,
                    penalty,
                    business_rules_log,
                })
            }
            Err(err) => {
                err_log.push(format!("Node {} failed business rule {}", touched_node.principal, err));
                None
            }
        }
    }

    /// Add nodes to a subnet in a way that provides the best decentralization.
    pub fn subnet_with_more_nodes(self, how_many_nodes: usize, available_nodes: &[Node], all_nodes: &[Node]) -> anyhow::Result<DecentralizedSubnet> {
        let mut run_log = self.run_log.clone();

        let mut nodes_initial = self.nodes.clone();
        let mut available_nodes = available_nodes.to_vec();
        let orig_available_nodes_len = &available_nodes.len();
        let mut nodes_after_extension = self.nodes.clone();
        let mut added_nodes = Vec::new();
        let mut comment = None;
        let mut total_penalty = 0;
        let mut business_rules_log: Vec<String> = Vec::new();

        run_log.push(format!("Nakamoto score before extension {}", self.nakamoto_score()));

        for i in 0..how_many_nodes {
            run_log.push("***********************************************************".to_string());
            run_log.push(format!("***  Adding node {}/{}", i + 1, how_many_nodes));
            run_log.push("***********************************************************".to_string());

            let suitable_candidates: Vec<ReplacementCandidate> = available_nodes
                .iter()
                .filter_map(|node| {
                    let subnet_nodes: Vec<Node> = nodes_initial.iter().chain([node]).cloned().collect();
                    self._node_to_replacement_candidate(&subnet_nodes, node, &mut run_log)
                })
                .collect();

            let mut candidate_run_log = Vec::new();
            match self.choose_best_candidate(suitable_candidates, &mut candidate_run_log, all_nodes) {
                Some(best_result) => {
                    run_log.extend(
                        candidate_run_log
                            .iter()
                            .map(|s| format!("node {}/{}: {}", i + 1, how_many_nodes, s))
                            .collect::<Vec<String>>(),
                    );
                    run_log.push(format!("Nakamoto score after extension {}", best_result.score));
                    added_nodes.push(best_result.node.clone());
                    available_nodes.retain(|n| n.principal != best_result.node.principal);
                    nodes_after_extension.push(best_result.node.clone());
                    nodes_initial.push(best_result.node.clone());
                    total_penalty += best_result.penalty;
                    business_rules_log.extend(
                        best_result
                            .business_rules_log
                            .iter()
                            .map(|s| {
                                format!(
                                    "- adding node {} of {} ({}): {}",
                                    i + 1,
                                    how_many_nodes,
                                    best_result.node.principal.to_string().split('-').next().unwrap_or_default(),
                                    s
                                )
                            })
                            .collect::<Vec<String>>(),
                    );
                    if i + 1 == how_many_nodes {
                        if total_penalty != 0 {
                            comment = Some(format!(
                                "Subnet extension with {} nodes finished with the total penalty {}. Penalty causes throughout the extension:\n\n{}\n\n{}",
                                how_many_nodes,
                                total_penalty,
                                business_rules_log.join("\n"),
                                if how_many_nodes > 1 {
                                    "Business rules analysis is calculated on each operation. Typically only the last operation is relevant, although this may depend on the case."
                                } else { "" }
                            ));
                        } else {
                            comment = None;
                        }
                    }
                }
                None => return Err(anyhow!("Could not complete the extension. Run log:\n{}", run_log.join("\n"))),
            }
        }

        assert_eq!(nodes_after_extension.len(), self.nodes.len() + how_many_nodes);
        assert_eq!(orig_available_nodes_len - available_nodes.len(), how_many_nodes);

        Ok(Self {
            id: self.id,
            nodes: nodes_after_extension,
            added_nodes: [self.added_nodes, added_nodes].concat(),
            removed_nodes: self.removed_nodes,
            comment,
            run_log,
        })
    }

    /// Remove nodes from a subnet in a way that provides the best decentralization.
    pub fn subnet_with_fewer_nodes(mut self, how_many_nodes: usize, all_nodes: &[Node]) -> anyhow::Result<DecentralizedSubnet> {
        let mut run_log = self.run_log.clone();
        let nodes_initial_len = self.nodes.len();
        let mut comment = None;
        let mut total_penalty = 0;
        let mut business_rules_log: Vec<String> = Vec::new();

        run_log.push(format!("Nakamoto score before removal {}", self.nakamoto_score()));

        for i in 0..how_many_nodes {
            run_log.push("***********************************************************".to_string());
            run_log.push(format!("***  Removing node {}/{}", i + 1, how_many_nodes));
            run_log.push("***********************************************************".to_string());

            let suitable_candidates: Vec<ReplacementCandidate> = self
                .nodes
                .iter()
                .filter_map(|node| {
                    let candidate_subnet_nodes: Vec<Node> = self.nodes.iter().filter(|n| n.principal != node.principal).cloned().collect();
                    self._node_to_replacement_candidate(&candidate_subnet_nodes, node, &mut run_log)
                })
                .collect();

            let mut candidate_run_log = Vec::new();
            match self.choose_best_candidate(suitable_candidates, &mut candidate_run_log, all_nodes) {
                Some(best_result) => {
                    run_log.extend(
                        candidate_run_log
                            .iter()
                            .map(|s| format!("node {}/{}: {}", i + 1, how_many_nodes, s))
                            .collect::<Vec<String>>(),
                    );
                    run_log.push(format!("Nakamoto score after removal {}", best_result.score));
                    self.removed_nodes.push(best_result.node.clone());
                    self.nodes.retain(|n| n.principal != best_result.node.principal);
                    total_penalty += best_result.penalty;
                    business_rules_log.extend(
                        best_result
                            .business_rules_log
                            .iter()
                            .map(|s| {
                                format!(
                                    "- removing node {} of {} ({}): {}",
                                    i + 1,
                                    how_many_nodes,
                                    best_result.node.principal.to_string().split('-').next().unwrap_or_default(),
                                    s
                                )
                            })
                            .collect::<Vec<String>>(),
                    );
                    if i + 1 == how_many_nodes {
                        if total_penalty != 0 {
                            comment = Some(format!(
                                "Subnet removal of {} nodes finished with the total penalty {}. Penalty causes throughout the removal:\n\n{}\n\n{}",
                                how_many_nodes,
                                total_penalty,
                                business_rules_log.join("\n"),
                                if how_many_nodes > 1 {
                                    "Business rules analysis is calculated on each operation. Typically only the last operation is relevant, although this may depend on the case."
                                } else {
                                    ""
                                }
                            ));
                        } else {
                            comment = None;
                        }
                    }
                }
                None => return Err(anyhow!("Could not complete the extension. Run log:\n{}", run_log.join("\n"))),
            }
        }
        assert_eq!(self.nodes.len(), nodes_initial_len - how_many_nodes);

        Ok(Self {
            id: self.id,
            nodes: self.nodes.clone(),
            added_nodes: self.added_nodes,
            removed_nodes: self.removed_nodes,
            comment,
            run_log,
        })
    }

    pub fn without_duplicate_added_removed(self) -> DecentralizedSubnet {
        let common_nodes: Vec<PrincipalId> = self
            .removed_nodes
            .iter()
            .filter_map(|node_removed| {
                if self.added_nodes.iter().any(|node_added| node_removed.principal == node_added.principal) {
                    Some(node_removed.principal)
                } else {
                    None
                }
            })
            .collect();

        if !common_nodes.is_empty() {
            info!("Removing nodes which have been removed and then added back: {:?}", common_nodes);

            let added_nodes_desc = self
                .added_nodes
                .into_iter()
                .filter(|node_added| !common_nodes.iter().any(|common_node| common_node == &node_added.principal))
                .collect();

            let removed_nodes_desc = self
                .removed_nodes
                .into_iter()
                .filter(|node_removed| !common_nodes.iter().any(|common_node| common_node == &node_removed.principal))
                .collect();

            Self {
                id: self.id,
                nodes: self.nodes.clone(),
                added_nodes: added_nodes_desc,
                removed_nodes: removed_nodes_desc,
                comment: self.comment.clone(),
                run_log: self.run_log.clone(),
            }
        } else {
            self
        }
    }
}

impl Display for DecentralizedSubnet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Subnet id {} with {} nodes [{}]",
            self.id,
            self.nodes.len(),
            self.nodes.iter().map(|n| n.principal.to_string()).join(", ")
        )
    }
}

impl From<DecentralizedSubnet> for NakamotoScore {
    fn from(subnet: DecentralizedSubnet) -> Self {
        Self::new_from_nodes(&subnet.nodes)
    }
}

impl From<&ic_management_types::Subnet> for DecentralizedSubnet {
    fn from(s: &ic_management_types::Subnet) -> Self {
        Self {
            id: s.principal,
            nodes: s.nodes.clone(),
            added_nodes: Vec::new(),
            removed_nodes: Vec::new(),
            comment: None,
            run_log: Vec::new(),
        }
    }
}

impl From<ic_management_types::Subnet> for DecentralizedSubnet {
    fn from(s: ic_management_types::Subnet) -> Self {
        Self::from(&s)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    impl ReplacementCandidate {
        pub fn new_with_node_for_tests(node: Node) -> Self {
            Self { node, ..Default::default() }
        }
    }
}
