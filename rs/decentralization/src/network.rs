use crate::nakamoto::{self, NakamotoScore};
use crate::SubnetChangeResponse;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use anyhow::anyhow;
use async_trait::async_trait;
use ic_base_types::PrincipalId;
use ic_management_types::{MinNakamotoCoefficients, NetworkError, NodeFeature};
use itertools::Itertools;
use log::{debug, info, warn};
use rand::{seq::SliceRandom, SeedableRng};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct DataCenterInfo {
    city: String,
    country: String,
    continent: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq)]
pub struct Node {
    pub id: PrincipalId,
    pub features: nakamoto::NodeFeatures,
    pub dfinity_owned: bool,
    pub decentralized: bool,
}

impl Node {
    pub fn new_test_node(node_number: u64, features: nakamoto::NodeFeatures, dfinity_owned: bool, decentralized: bool) -> Self {
        Node {
            id: PrincipalId::new_node_test_id(node_number),
            features,
            dfinity_owned,
            decentralized,
        }
    }

    pub fn get_features(&self) -> nakamoto::NodeFeatures {
        self.features.clone()
    }

    pub fn get_feature(&self, feature: &NodeFeature) -> String {
        self.features.get(feature).unwrap_or_default()
    }

    pub fn matches_feature_value(&self, value: &str) -> bool {
        self.id.to_string() == *value.to_lowercase()
            || self
                .get_features()
                .feature_map
                .values()
                .any(|v| *v.to_lowercase() == *value.to_lowercase())
    }
}

impl Hash for Node {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl From<&ic_management_types::Node> for Node {
    fn from(n: &ic_management_types::Node) -> Self {
        Self {
            id: n.principal,
            features: nakamoto::NodeFeatures::from_iter([
                (
                    NodeFeature::City,
                    n.operator
                        .datacenter
                        .as_ref()
                        .map(|d| d.city.clone())
                        .unwrap_or_else(|| "unknown".to_string()),
                ),
                (
                    NodeFeature::Country,
                    n.operator
                        .datacenter
                        .as_ref()
                        .map(|d| d.country.clone())
                        .unwrap_or_else(|| "unknown".to_string()),
                ),
                (
                    NodeFeature::Continent,
                    n.operator
                        .datacenter
                        .as_ref()
                        .map(|d| d.continent.clone())
                        .unwrap_or_else(|| "unknown".to_string()),
                ),
                (
                    NodeFeature::DataCenterOwner,
                    n.operator
                        .datacenter
                        .as_ref()
                        .map(|d| d.owner.name.clone())
                        .unwrap_or_else(|| "unknown".to_string()),
                ),
                (
                    NodeFeature::DataCenter,
                    n.operator
                        .datacenter
                        .as_ref()
                        .map(|d| d.name.clone())
                        .unwrap_or_else(|| "unknown".to_string()),
                ),
                (NodeFeature::NodeProvider, n.operator.provider.principal.to_string()),
            ]),
            dfinity_owned: n.dfinity_owned.unwrap_or_default(),
            decentralized: n.decentralized,
        }
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecentralizedSubnet {
    pub id: PrincipalId,
    pub nodes: Vec<Node>,
    pub removed_nodes: Vec<Node>,
    pub min_nakamoto_coefficients: Option<MinNakamotoCoefficients>,
    pub comment: Option<String>,
    pub run_log: Vec<String>,
}

#[derive(Clone, Debug)]
struct ReplacementCandidate {
    node: Node,
    score: NakamotoScore,
    penalty: usize,
    business_rules_log: Vec<String>,
}

impl DecentralizedSubnet {
    pub fn with_subnet_id(self, subnet_id: PrincipalId) -> Self {
        Self { id: subnet_id, ..self }
    }

    /// Return a new instance of a DecentralizedSubnet that does not contain the
    /// provided nodes.
    pub fn without_nodes(&self, nodes: Vec<Node>) -> Result<Self, NetworkError> {
        let mut new_subnet_nodes = self.nodes.clone();
        let mut removed = Vec::new();
        for node in &nodes {
            if let Some(index) = new_subnet_nodes.iter().position(|n| n.id == node.id) {
                removed.push(new_subnet_nodes.remove(index));
            } else {
                return Err(NetworkError::NodeNotFound(node.id));
            }
        }
        let removed_is_empty = removed.is_empty();
        let removed_node_ids = removed.iter().map(|n| n.id).collect::<Vec<_>>();
        Ok(Self {
            id: self.id,
            nodes: new_subnet_nodes,
            removed_nodes: removed,
            min_nakamoto_coefficients: self.min_nakamoto_coefficients.clone(),
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
    pub fn with_nodes(self, nodes: Vec<Node>) -> Self {
        Self {
            id: self.id,
            nodes: self.nodes.clone().into_iter().chain(nodes.clone()).collect(),
            removed_nodes: self.removed_nodes,
            min_nakamoto_coefficients: self.min_nakamoto_coefficients.clone(),
            comment: self.comment,
            run_log: {
                if nodes.is_empty() {
                    self.run_log
                } else {
                    let mut run_log = self.run_log;
                    run_log.push(format!("Force-including nodes {:?}", nodes.iter().map(|n| n.id).collect::<Vec<_>>()));
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
            .filter(|n| n.get_feature(node_feature) == dominant_feature)
            .cloned()
            .collect()
    }

    pub fn with_min_nakamoto_coefficients(self, min_nakamoto_coefficients: &Option<MinNakamotoCoefficients>) -> Self {
        Self {
            min_nakamoto_coefficients: min_nakamoto_coefficients.clone(),
            ..self
        }
    }

    /// Ensure "business rules" or constraints for the subnet nodes are met.
    /// For instance, there needs to be at least one DFINITY-owned node in each
    /// subnet. For the mainnet NNS there needs to be at least 3
    /// DFINITY-owned nodes.
    pub fn check_business_rules(&self) -> anyhow::Result<(usize, Vec<String>)> {
        Self::_check_business_rules_for_nodes(&self.id, &self.nodes, &self.min_nakamoto_coefficients)
    }

    fn _check_business_rules_for_nodes(
        subnet_id: &PrincipalId,
        nodes: &[Node],
        min_nakamoto_coefficients: &Option<MinNakamotoCoefficients>,
    ) -> anyhow::Result<(usize, Vec<String>)> {
        let mut checks = Vec::new();
        let mut penalties = 0;
        if nodes.len() <= 1 {
            return Ok((1, checks));
        }

        let nakamoto_scores = Self::_calc_nakamoto_score(nodes);
        let subnet_id_str = subnet_id.to_string();
        let is_european_subnet = subnet_id_str == *"bkfrj-6k62g-dycql-7h53p-atvkj-zg4to-gaogh-netha-ptybj-ntsgw-rqe";

        let dfinity_owned_nodes_count: usize = nodes.iter().map(|n| n.dfinity_owned as usize).sum();
        let target_dfinity_owned_nodes_count = if subnet_id_str == *"tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe" {
            3
        } else {
            1
        };

        if dfinity_owned_nodes_count != target_dfinity_owned_nodes_count {
            checks.push(format!(
                "Subnet should have {} DFINITY-owned nodes, got {}",
                target_dfinity_owned_nodes_count, dfinity_owned_nodes_count
            ));
            penalties += target_dfinity_owned_nodes_count.abs_diff(dfinity_owned_nodes_count) * 1000;
        }

        let count_non_decentralized_nodes = nodes.iter().filter(|n| !n.decentralized).count();
        if count_non_decentralized_nodes > 0 {
            checks.push(format!("Subnet has {} non-decentralized node(s)", count_non_decentralized_nodes));
            penalties += count_non_decentralized_nodes * 100;
        }

        if subnet_id_str == *"uzr34-akd3s-xrdag-3ql62-ocgoh-ld2ao-tamcv-54e7j-krwgb-2gm4z-oqe"
            || subnet_id_str == *"tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe"
            || subnet_id_str == *"x33ed-h457x-bsgyx-oqxqf-6pzwv-wkhzr-rm2j3-npodi-purzm-n66cg-gae"
        {
            // We keep the backup of the ECDSA key on uzr34, and we don’t want a single
            // country to be able to extract that key.
            // The tECDSA key can be extracted with 1/3 of the nodes.
            // We should use the same NC requirements for uzr34 and the upcoming ECDSA
            // subnet, since they'll both hold the same valuable key.
            // Slack discussion: https://dfinity.slack.com/archives/C01DB8MQ5M1/p1668702249558389
            // For different reasons, there is the same requirement for the NNS and the SNS
            // subnet.
            let feature = NodeFeature::Country;
            match nakamoto_scores.feature_value_counts_max(&feature) {
                Some((country_dominant, country_nodes_count)) => {
                    let controlled_nodes_max = nodes.len() / 3;
                    if country_nodes_count > controlled_nodes_max {
                        checks.push(format!(
                            "Country '{}' controls {} of nodes, which is > {} (1/3 - 1) of subnet nodes",
                            country_dominant, country_nodes_count, controlled_nodes_max
                        ));
                        penalties += (country_nodes_count - controlled_nodes_max) * 1000;
                    }
                }
                _ => return Err(anyhow::anyhow!("Incomplete data for {}", feature)),
            }
        }

        if is_european_subnet {
            // European subnet should only take European nodes.
            let continent_counts = nakamoto_scores.feature_value_counts(&NodeFeature::Continent);
            let non_european_nodes_count = continent_counts
                .iter()
                .filter_map(|(continent, count)| if continent == &"Europe".to_string() { None } else { Some(*count) })
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

        if let Some(min_nakamoto_coefficients) = min_nakamoto_coefficients {
            for (feature, min_coeff) in min_nakamoto_coefficients.coefficients.iter() {
                match nakamoto_scores.score_feature(feature) {
                    Some(score) => {
                        if score < *min_coeff {
                            checks.push(format!(
                                "Lower than expected Nakamoto Coefficient {} < {} for feature {}",
                                score, min_coeff, feature
                            ));
                            penalties += ((*min_coeff - score) * 100.) as usize;
                        }
                    }
                    None => return Err(anyhow::anyhow!("NodeFeature '{}' not found", feature.to_string())),
                }
            }
            if nakamoto_scores.score_avg_linear() < min_nakamoto_coefficients.average {
                checks.push(format!(
                    "Lower than expected average Nakamoto Coefficient {} < {}",
                    nakamoto_scores.score_avg_linear(),
                    min_nakamoto_coefficients.average
                ));
                penalties += ((min_nakamoto_coefficients.average - nakamoto_scores.score_avg_linear()) * 100.) as usize;
            }
        }

        for feature in &NodeFeature::variants() {
            match (nakamoto_scores.score_feature(feature), nakamoto_scores.controlled_nodes(feature)) {
                (Some(score), Some(controlled_nodes)) => {
                    let european_subnet_continent_penalty = is_european_subnet && feature == &NodeFeature::Continent;

                    if score == 1.0 && controlled_nodes > nodes.len() * 2 / 3 && !european_subnet_continent_penalty {
                        checks.push(format!(
                            "NodeFeature '{}' controls {} of nodes, which is > {} (2/3 of all) nodes",
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
    fn choose_deterministic_random(best_results: &Vec<ReplacementCandidate>, current_nodes: &[Node]) -> Option<ReplacementCandidate> {
        if best_results.is_empty() {
            None
        } else {
            // We sort the current nodes by alphabetical order on their
            // PrincipalIDs to ensure consistency of the seed with the
            // same machines in the subnet
            let mut id_sorted_current_nodes = current_nodes.to_owned();
            id_sorted_current_nodes.sort_by(|n1, n2| std::cmp::Ord::cmp(&n1.id.to_string(), &n2.id.to_string()));
            let seed = rand_seeder::Seeder::from(
                id_sorted_current_nodes
                    .iter()
                    .map(|n| n.id.to_string())
                    .collect::<Vec<String>>()
                    .join("_"),
            )
            .make_seed();
            let mut rng = rand::rngs::StdRng::from_seed(seed);

            // We sort the best results the same way to ensure that for
            // the same set of machines with the best score, we always
            // get the same one.
            let mut id_sorted_best_results = best_results.clone();
            id_sorted_best_results.sort_by(|r1, r2| std::cmp::Ord::cmp(&r1.node.id.to_string(), &r2.node.id.to_string()));
            id_sorted_best_results.choose(&mut rng).cloned()
        }
    }

    /// Pick the best result amongst the list of "suitable" candidates.
    fn choose_best_candidate(&self, candidates: Vec<ReplacementCandidate>, run_log: &mut Vec<String>) -> Option<ReplacementCandidate> {
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
                    debug!("Better node is {}", a.node.id);
                } else {
                    debug!("Better node is {}", b.node.id);
                }
                cmp
            })
            .collect::<Vec<ReplacementCandidate>>();

        run_log.push("Sorted candidate nodes, with the best candidate at the end:".to_string());
        run_log.push("     <node-id>                                                      <penalty>  <Nakamoto score>".to_string());
        for s in &candidates {
            run_log.push(format!(" -=> {} {} {}", s.node.id, s.penalty, s.score));
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
        DecentralizedSubnet::choose_deterministic_random(&best_results, &self.nodes)
    }

    /// Add nodes to a subnet in a way that provides the best decentralization.
    pub fn subnet_with_more_nodes(self, how_many_nodes: usize, available_nodes: &[Node]) -> anyhow::Result<DecentralizedSubnet> {
        let mut run_log = self.run_log.clone();

        let mut nodes_initial = self.nodes.clone();
        let mut available_nodes = available_nodes.to_vec();
        let orig_available_nodes_len = &available_nodes.len();
        let mut nodes_after_extension = self.nodes.clone();
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
            match self.choose_best_candidate(suitable_candidates, &mut candidate_run_log) {
                Some(best_result) => {
                    // Append the complete run log
                    run_log.extend(
                        candidate_run_log
                            .iter()
                            .map(|s| format!("node {}/{}: {}", i + 1, how_many_nodes, s))
                            .collect::<Vec<String>>(),
                    );
                    run_log.push(format!("Nakamoto score after extension {}", best_result.score));
                    available_nodes.retain(|n| n.id != best_result.node.id);
                    nodes_after_extension.push(best_result.node.clone());
                    nodes_initial.push(best_result.node.clone());
                    total_penalty += best_result.penalty;
                    business_rules_log.extend(
                        best_result
                            .business_rules_log
                            .iter()
                            .map(|s| format!("node {}/{} ({}): {}", i + 1, how_many_nodes, best_result.node.id, s))
                            .collect::<Vec<String>>(),
                    );
                    if i + 1 == how_many_nodes {
                        if total_penalty != 0 {
                            comment = Some(format!(
                                "Subnet extension with {} nodes finished with the total penalty {}. Penalty causes throughout the extension:\n{}\n\n{}",
                                how_many_nodes,
                                total_penalty,
                                business_rules_log.join("\n"),
                                if how_many_nodes > 1 {
                                    "Note that the penalty for nodes before the last node may not be relevant in the end. We leave this to humans to assess."
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
            removed_nodes: self.removed_nodes,
            min_nakamoto_coefficients: self.min_nakamoto_coefficients,
            comment,
            run_log,
        })
    }

    /// Remove nodes from a subnet in a way that provides the best
    /// decentralization.
    pub fn subnet_with_fewer_nodes(mut self, how_many_nodes: usize) -> anyhow::Result<DecentralizedSubnet> {
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
                    let candidate_subnet_nodes: Vec<Node> = self.nodes.iter().filter(|n| n.id != node.id).cloned().collect();
                    self._node_to_replacement_candidate(&candidate_subnet_nodes, node, &mut run_log)
                })
                .collect();

            let mut candidate_run_log = Vec::new();
            match self.choose_best_candidate(suitable_candidates, &mut candidate_run_log) {
                Some(best_result) => {
                    // Append the complete run log
                    run_log.extend(
                        candidate_run_log
                            .iter()
                            .map(|s| format!("node {}/{}: {}", i + 1, how_many_nodes, s))
                            .collect::<Vec<String>>(),
                    );
                    run_log.push(format!("Nakamoto score after removal {}", best_result.score));
                    self.removed_nodes.push(best_result.node.clone());
                    self.nodes.retain(|n| n.id != best_result.node.id);
                    total_penalty += best_result.penalty;
                    business_rules_log.extend(
                        best_result
                            .business_rules_log
                            .iter()
                            .map(|s| format!("node {}/{} ({}): {}", i + 1, how_many_nodes, best_result.node.id, s))
                            .collect::<Vec<String>>(),
                    );
                    if i + 1 == how_many_nodes {
                        if total_penalty != 0 {
                            comment = Some(format!(
                                "Subnet removal of {} nodes finished with the total penalty {}. Penalty causes throughout the removal:\n{}\n\n{}",
                                how_many_nodes,
                                total_penalty,
                                business_rules_log.join("\n"),
                                if how_many_nodes > 1 {
                                    "Note that the penalty for nodes before the last node may not be relevant in the end. We leave this to humans to assess."
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
            removed_nodes: self.removed_nodes,
            min_nakamoto_coefficients: self.min_nakamoto_coefficients,
            comment,
            run_log,
        })
    }

    fn _node_to_replacement_candidate(&self, subnet_nodes: &[Node], touched_node: &Node, err_log: &mut Vec<String>) -> Option<ReplacementCandidate> {
        match Self::_check_business_rules_for_nodes(&self.id, subnet_nodes, &self.min_nakamoto_coefficients) {
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
                err_log.push(format!("Node {} failed business rule {}", touched_node.id, err));
                None
            }
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
            self.nodes.iter().map(|n| n.id.to_string()).join(", ")
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
            nodes: s.nodes.iter().map(Node::from).collect(),
            removed_nodes: Vec::new(),
            min_nakamoto_coefficients: None,
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

#[async_trait]
pub trait AvailableNodesQuerier {
    async fn available_nodes(&self) -> Result<Vec<Node>, NetworkError>;
}

pub enum SubnetQueryBy {
    SubnetId(PrincipalId),
    NodeList(Vec<Node>),
}

#[async_trait]
pub trait SubnetQuerier {
    async fn subnet(&self, by: SubnetQueryBy) -> Result<DecentralizedSubnet, NetworkError>;
}

#[derive(Clone, Serialize, Deserialize, Debug, strum_macros::Display)]
pub enum DecentralizationError {
    FeatureNotAvailable,
}

impl ResponseError for DecentralizationError {
    fn error_response(&self) -> HttpResponse {
        let out: serde_json::Value =
            serde_json::from_str("{\"message\": \"NodeFeature not available. For access contact the administrator\"}").unwrap();
        HttpResponse::BadRequest().json(out)
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::FeatureNotAvailable => StatusCode::BAD_REQUEST,
        }
    }
}

#[async_trait]
pub trait TopologyManager: SubnetQuerier + AvailableNodesQuerier {
    async fn modify_subnet_nodes(&self, by: SubnetQueryBy) -> Result<SubnetChangeRequest, NetworkError> {
        Ok(SubnetChangeRequest {
            available_nodes: self.available_nodes().await?,
            subnet: self.subnet(by).await?,
            ..Default::default()
        })
    }

    async fn create_subnet(
        &self,
        size: usize,
        min_nakamoto_coefficients: Option<MinNakamotoCoefficients>,
        include_nodes: Vec<PrincipalId>,
        exclude_nodes: Vec<String>,
        only_nodes: Vec<String>,
    ) -> Result<SubnetChange, NetworkError> {
        SubnetChangeRequest {
            available_nodes: self.available_nodes().await?,
            min_nakamoto_coefficients,
            ..Default::default()
        }
        .including_from_available(NodeSelector::PrincipalIdList(include_nodes.clone()))
        .excluding_from_available(NodeSelector::FeatureList(exclude_nodes.clone()))
        .including_from_available(NodeSelector::FeatureList(only_nodes.clone()))
        .resize(size, 0)
    }
}

pub enum NodeSelector {
    PrincipalIdList(Vec<PrincipalId>),
    NodeList(Vec<Node>),
    FeatureList(Vec<String>),
}

impl NodeSelector {
    pub fn partition(&self, nodes: Vec<Node>) -> (Vec<Node>, Vec<Node>) {
        nodes.into_iter().partition(|node: &Node| match &self {
            NodeSelector::PrincipalIdList(list) => list.contains(&node.id),
            NodeSelector::NodeList(list) => list.contains(node),
            NodeSelector::FeatureList(list) => list.iter().any(|v| node.matches_feature_value(v)),
        })
    }
}

#[derive(Default, Clone, Debug)]
pub struct SubnetChangeRequest {
    subnet: DecentralizedSubnet,
    available_nodes: Vec<Node>,
    include_nodes: Vec<Node>,
    removed_nodes: Vec<Node>,
    min_nakamoto_coefficients: Option<MinNakamotoCoefficients>,
}

impl SubnetChangeRequest {
    pub fn new(
        subnet: DecentralizedSubnet,
        available_nodes: Vec<Node>,
        include_nodes: Vec<Node>,
        removed_nodes: Vec<Node>,
        min_nakamoto_coefficients: Option<MinNakamotoCoefficients>,
    ) -> Self {
        SubnetChangeRequest {
            subnet,
            available_nodes,
            include_nodes,
            removed_nodes,
            min_nakamoto_coefficients,
        }
    }

    pub fn subnet(&self) -> DecentralizedSubnet {
        self.subnet.clone()
    }

    pub fn with_custom_available_nodes(self, nodes: Vec<Node>) -> Self {
        Self {
            available_nodes: nodes,
            ..self
        }
    }

    pub fn including_from_available(self, selector: NodeSelector) -> Self {
        let (selected, _) = selector.partition(self.available_nodes.clone());
        Self {
            include_nodes: selected,
            ..self
        }
    }

    pub fn excluding_from_available(self, selector: NodeSelector) -> Self {
        let (_, unselected) = selector.partition(self.available_nodes.clone());
        Self {
            available_nodes: unselected,
            ..self
        }
    }

    pub fn keeping_from_used(self, selector: NodeSelector) -> Self {
        let mut change_new = self.clone();
        let (selected, unselected) = selector.partition(self.subnet.nodes);
        change_new.subnet.nodes = selected;
        change_new.removed_nodes.extend(unselected);

        change_new
    }

    pub fn removing_from_used(self, selector: NodeSelector) -> Self {
        let mut change_new = self.clone();
        let (selected, unselected) = selector.partition(self.subnet.nodes);
        change_new.subnet.nodes = unselected;
        change_new.removed_nodes.extend(selected);

        change_new
    }

    pub fn with_min_nakamoto_coefficients(self, min_nakamoto_coefficients: Option<MinNakamotoCoefficients>) -> Self {
        Self {
            min_nakamoto_coefficients,
            ..self
        }
    }

    /// Optimize is implemented by removing a certain number of nodes and then
    /// adding the same number back.
    pub fn optimize(mut self, optimize_count: usize, replacements_unhealthy: &Vec<Node>) -> Result<SubnetChange, NetworkError> {
        let old_nodes = self.subnet.nodes.clone();
        self.subnet = self.subnet.without_nodes(replacements_unhealthy.clone())?;
        let result = self.resize(optimize_count + replacements_unhealthy.len(), optimize_count)?;
        Ok(SubnetChange { old_nodes, ..result })
    }

    pub fn rescue(&self) -> Result<SubnetChange, NetworkError> {
        let old_nodes = self.subnet.nodes.iter().chain(self.removed_nodes.iter()).cloned().collect_vec();
        let result = self.resize(self.removed_nodes.len(), 0)?;
        Ok(SubnetChange { old_nodes, ..result })
    }

    /// Add or remove nodes from the subnet.
    pub fn resize(&self, how_many_nodes_to_add: usize, how_many_nodes_to_remove: usize) -> Result<SubnetChange, NetworkError> {
        println!(
            "Resizing subnet {} by adding {} nodes and removing {} nodes",
            self.subnet.id, how_many_nodes_to_add, how_many_nodes_to_remove
        );
        let old_nodes = self.subnet.nodes.clone();

        let available_nodes = self
            .available_nodes
            .clone()
            .into_iter()
            .filter(|n| !self.include_nodes.contains(n))
            .collect::<Vec<_>>();

        println!("available {}", available_nodes.len());

        let resized_subnet = self
            .subnet
            .clone()
            .with_nodes(self.include_nodes.clone())
            .with_min_nakamoto_coefficients(&self.min_nakamoto_coefficients)
            .subnet_with_more_nodes(how_many_nodes_to_add, &available_nodes)
            .map_err(|e| NetworkError::ResizeFailed(e.to_string()))?;

        let resized_subnet = if how_many_nodes_to_remove > 0 {
            resized_subnet
                .subnet_with_fewer_nodes(how_many_nodes_to_remove)
                .map_err(|e| NetworkError::ResizeFailed(e.to_string()))?
        } else {
            resized_subnet
        };

        let subnet_change = SubnetChange {
            id: self.subnet.id,
            old_nodes,
            new_nodes: resized_subnet.nodes,
            min_nakamoto_coefficients: self.min_nakamoto_coefficients.clone(),
            comment: resized_subnet.comment,
            run_log: resized_subnet.run_log,
        };
        let node_add_count = subnet_change.added().len();
        let node_remove_count = subnet_change.removed().len();
        info!(
            "Subnet {} resized, {} nodes added, {} nodes removed",
            self.subnet.id, node_add_count, node_remove_count
        );
        Ok(subnet_change)
    }

    /// Evaluates the subnet change request to simulate the requested topology
    /// change. Command returns all the information about the subnet before
    /// and after the change.
    pub fn evaluate(self) -> Result<SubnetChange, NetworkError> {
        self.resize(0, 0)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SubnetChange {
    pub id: PrincipalId,
    pub old_nodes: Vec<Node>,
    pub new_nodes: Vec<Node>,
    pub min_nakamoto_coefficients: Option<MinNakamotoCoefficients>,
    pub comment: Option<String>,
    pub run_log: Vec<String>,
}

impl SubnetChange {
    pub fn with_nodes(self, nodes: Vec<Node>) -> Self {
        Self {
            new_nodes: [self.new_nodes, nodes].concat(),
            ..self
        }
    }
    pub fn without_nodes(mut self, nodes: Vec<Node>) -> Self {
        self.new_nodes.retain(|n| !nodes.contains(n));
        self
    }

    pub fn added(&self) -> Vec<Node> {
        self.new_nodes.clone().into_iter().filter(|n| !self.old_nodes.contains(n)).collect()
    }

    pub fn removed(&self) -> Vec<Node> {
        self.old_nodes.clone().into_iter().filter(|n| !self.new_nodes.contains(n)).collect()
    }

    pub fn before(&self) -> DecentralizedSubnet {
        DecentralizedSubnet {
            id: self.id,
            nodes: self.old_nodes.clone(),
            removed_nodes: Vec::new(),
            min_nakamoto_coefficients: self.min_nakamoto_coefficients.clone(),
            comment: self.comment.clone(),
            run_log: Vec::new(),
        }
    }

    pub fn after(&self) -> DecentralizedSubnet {
        DecentralizedSubnet {
            id: self.id,
            nodes: self.new_nodes.clone(),
            removed_nodes: self.old_nodes.clone().into_iter().filter(|n| !self.new_nodes.contains(n)).collect(),
            min_nakamoto_coefficients: self.min_nakamoto_coefficients.clone(),
            comment: self.comment.clone(),
            run_log: self.run_log.clone(),
        }
    }
}

impl Display for SubnetChange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", SubnetChangeResponse::from(self))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkHealSubnets {
    pub name: String,
    pub decentralized_subnet: DecentralizedSubnet,
    pub unhealthy_nodes: Vec<Node>,
}

impl NetworkHealSubnets {
    const IMPORTANT_SUBNETS: &'static [&'static str] = &["NNS", "SNS", "Bitcoin", "Internet Identity", "tECDSA signing"];
}

impl PartialOrd for NetworkHealSubnets {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NetworkHealSubnets {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_is_important = NetworkHealSubnets::IMPORTANT_SUBNETS.contains(&self.name.as_str());
        let other_is_important = NetworkHealSubnets::IMPORTANT_SUBNETS.contains(&other.name.as_str());

        match (self_is_important, other_is_important) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            _ => self.decentralized_subnet.nodes.len().cmp(&other.decentralized_subnet.nodes.len()),
        }
    }
}

pub struct NetworkHealRequest {
    pub subnets: Vec<NetworkHealSubnets>,
}

impl NetworkHealRequest {
    pub fn new(subnets: Vec<NetworkHealSubnets>) -> Self {
        Self { subnets }
    }

    pub fn heal_and_optimize(
        &self,
        mut available_nodes: Vec<Node>,
        max_replaceable_nodes: Option<usize>,
    ) -> Result<Vec<SubnetChangeResponse>, NetworkError> {
        let mut subnets_changed = Vec::new();
        let subnets_to_heal = self.subnets.iter().sorted_by(|a, b| a.cmp(b).reverse()).collect_vec();

        for subnet in subnets_to_heal {
            let mut unhealthy_nodes = subnet.unhealthy_nodes.clone();
            let unhealthy_nodes_len = unhealthy_nodes.len();

            if let Some(max_replaceable_nodes) = max_replaceable_nodes {
                if unhealthy_nodes_len > max_replaceable_nodes {
                    unhealthy_nodes = subnet.unhealthy_nodes.clone().into_iter().take(max_replaceable_nodes).collect_vec();

                    warn!(
                        "Subnet {}: replacing {} of {} unhealthy nodes: {:?}",
                        subnet.decentralized_subnet.id,
                        max_replaceable_nodes,
                        unhealthy_nodes_len,
                        unhealthy_nodes.iter().map(|node| node.id).collect_vec()
                    );
                }
            }
            let unhealthy_nodes_len = unhealthy_nodes.len();
            let optimize_limit = max_replaceable_nodes.unwrap_or(unhealthy_nodes_len) - unhealthy_nodes_len;

            let change = SubnetChangeRequest {
                subnet: subnet.decentralized_subnet.clone(),
                available_nodes: available_nodes.clone(),
                ..Default::default()
            };
            let change = change.optimize(optimize_limit, &unhealthy_nodes)?;

            available_nodes.retain(|node| !change.added().contains(node));
            subnets_changed.push(SubnetChangeResponse::from(&change));
        }

        Ok(subnets_changed)
    }
}
