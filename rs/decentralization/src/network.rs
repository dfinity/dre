use crate::nakamoto::{self, NakamotoScore};
use crate::subnets::unhealthy_with_nodes;
use crate::SubnetChangeResponse;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use ahash::{AHashMap, AHashSet, HashMap, HashSet};
use anyhow::anyhow;
use futures::future::BoxFuture;
use ic_base_types::PrincipalId;
use ic_management_types::{HealthStatus, NetworkError, NodeFeature};
use indexmap::IndexMap;
use itertools::Itertools;
use log::{debug, info, warn};
use rand::{seq::SliceRandom, SeedableRng};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct DataCenterInfo {
    area: String,
    country: String,
    continent: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq)]
pub struct Node {
    pub id: PrincipalId,
    pub features: nakamoto::NodeFeatures,
    pub dfinity_owned: bool,
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Node ID: {}\nFeatures:\n{}\nDfinity Owned: {}",
            self.id, self.features, self.dfinity_owned
        )
    }
}

impl Node {
    pub fn new_test_node(node_number: u64, features: nakamoto::NodeFeatures, dfinity_owned: bool) -> Self {
        Node {
            id: PrincipalId::new_node_test_id(node_number),
            features,
            dfinity_owned,
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

    pub fn is_country_from_eu(country: &str) -> bool {
        // (As of 2024) the EU countries are not properly marked in the registry, so we check membership separately.
        let eu_countries: HashMap<&str, &str> = HashMap::from_iter([
            ("AT", "Austria"),
            ("BE", "Belgium"),
            ("BG", "Bulgaria"),
            ("CY", "Cyprus"),
            ("CZ", "Czechia"),
            ("DE", "Germany"),
            ("DK", "Denmark"),
            ("EE", "Estonia"),
            ("ES", "Spain"),
            ("FI", "Finland"),
            ("FR", "France"),
            ("GR", "Greece"),
            ("HR", "Croatia"),
            ("HU", "Hungary"),
            ("IE", "Ireland"),
            ("IT", "Italy"),
            ("LT", "Lithuania"),
            ("LU", "Luxembourg"),
            ("LV", "Latvia"),
            ("MT", "Malta"),
            ("NL", "Netherlands"),
            ("PL", "Poland"),
            ("PT", "Portugal"),
            ("RO", "Romania"),
            ("SE", "Sweden"),
            ("SI", "Slovenia"),
            ("SK", "Slovakia"),
        ]);
        eu_countries.contains_key(country)
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
        let country = n
            .operator
            .datacenter
            .as_ref()
            .map(|d| d.country.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let area = n
            .operator
            .datacenter
            .as_ref()
            .map(|d| d.area.clone())
            .unwrap_or_else(|| "unknown".to_string());

        Self {
            id: n.principal,
            features: nakamoto::NodeFeatures::from_iter([
                (NodeFeature::Area, area),
                (NodeFeature::Country, country),
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
        }
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecentralizedSubnet {
    pub id: PrincipalId,
    pub nodes: Vec<Node>,
    pub added_nodes_desc: Vec<(Node, String)>,
    pub removed_nodes_desc: Vec<(Node, String)>,
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
    pub fn new_with_subnet_id_and_nodes(subnet_id: PrincipalId, nodes: Vec<Node>) -> Self {
        Self {
            id: subnet_id,
            nodes,
            added_nodes_desc: vec![],
            removed_nodes_desc: vec![],
            comment: None,
            run_log: vec![],
        }
    }

    pub fn with_subnet_id(self, subnet_id: PrincipalId) -> Self {
        Self { id: subnet_id, ..self }
    }

    /// Return a new instance of a DecentralizedSubnet that does not contain the
    /// provided nodes.
    pub fn without_nodes(&self, nodes_to_remove_with_desc: Vec<(Node, String)>) -> Result<Self, NetworkError> {
        let mut new_subnet_nodes = self.nodes.clone();
        let mut removed = Vec::new();
        for (node, desc) in &nodes_to_remove_with_desc {
            if let Some(index) = new_subnet_nodes.iter().position(|n| n.id == node.id) {
                removed.push((new_subnet_nodes.remove(index), desc));
            } else {
                return Err(NetworkError::NodeNotFound(node.id));
            }
        }
        let removed_is_empty = removed.is_empty();
        let removed_node_ids = removed.iter().map(|(n, _)| n.id).collect::<Vec<_>>();
        if !removed_is_empty {
            assert!(new_subnet_nodes.len() < self.nodes.len());
        }
        Ok(Self {
            id: self.id,
            nodes: new_subnet_nodes,
            added_nodes_desc: self.added_nodes_desc.clone(),
            removed_nodes_desc: removed.iter().map(|(n, desc)| (n.clone(), desc.to_string())).collect(),
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
    pub fn with_nodes(self, nodes_to_add_with_desc: Vec<(Node, String)>) -> Self {
        let new_subnet_nodes: Vec<Node> = self
            .nodes
            .clone()
            .into_iter()
            .chain(nodes_to_add_with_desc.iter().map(|(n, _)| n.clone()))
            .collect();
        if !nodes_to_add_with_desc.is_empty() {
            assert!(new_subnet_nodes.len() > self.nodes.len());
        }
        Self {
            id: self.id,
            nodes: new_subnet_nodes,
            added_nodes_desc: nodes_to_add_with_desc.clone(),
            removed_nodes_desc: self.removed_nodes_desc,
            comment: self.comment,
            run_log: {
                if nodes_to_add_with_desc.is_empty() {
                    self.run_log
                } else {
                    let mut run_log = self.run_log;
                    run_log.push(format!(
                        "Including user-provided nodes {:?}",
                        nodes_to_add_with_desc
                            .iter()
                            .map(|(n, desc)| format!("{}:{}", n, desc))
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
            .filter(|n| n.get_feature(node_feature) == dominant_feature)
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
                        let penalty = (country_nodes_count - controlled_nodes_max) * 1000;
                        checks.push(format!(
                            "Country {} controls {} of nodes, which is > {} (1/3 - 1) of subnet nodes. Applying penalty of {}.",
                            country_dominant, country_nodes_count, controlled_nodes_max, penalty
                        ));
                        penalties += penalty;
                    }
                }
                _ => return Err(anyhow::anyhow!("Incomplete data for {}", feature)),
            }
        }

        // As per the adopted target topology
        // https://dashboard.internetcomputer.org/proposal/132136
        let max_nodes_per_np_and_dc = 1;
        for feature in &[NodeFeature::NodeProvider, NodeFeature::DataCenter, NodeFeature::DataCenterOwner] {
            match nakamoto_scores.feature_value_counts_max(feature) {
                Some((name, value)) => {
                    if value > max_nodes_per_np_and_dc {
                        let penalty = (value - max_nodes_per_np_and_dc) * 10;
                        checks.push(format!(
                            "{} {} controls {} of nodes, which is higher than target of {} for the subnet. Applying penalty of {}.",
                            feature, name, value, max_nodes_per_np_and_dc, penalty
                        ));
                        penalties += penalty;
                    }
                }
                _ => return Err(anyhow::anyhow!("Incomplete data for {}", feature)),
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
        match nakamoto_scores.feature_value_counts_max(&NodeFeature::Country) {
            Some((name, value)) => {
                if is_european_subnet && !Node::is_country_from_eu(name.as_str()) {
                    // European subnet is expected to be controlled by European countries
                } else if value > max_nodes_per_country {
                    let penalty = (value - max_nodes_per_country) * 10;
                    checks.push(format!(
                        "Country {} controls {} of nodes, which is higher than target of {} for the subnet. Applying penalty of {}.",
                        name, value, max_nodes_per_country, penalty
                    ));
                    penalties += penalty;
                }
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Incomplete data for Node Feature Country in subnet {}",
                    subnet_id.to_string()
                ))
            }
        }

        if is_european_subnet {
            // European subnet should only take European nodes.
            let country_counts = nakamoto_scores.feature_value_counts(&NodeFeature::Country);
            let non_european_nodes_count = country_counts
                .iter()
                .filter_map(|(country, count)| {
                    if Node::is_country_from_eu(country.as_str()) || country.as_str() == "CH" {
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
    fn choose_deterministic_random(best_results: &[ReplacementCandidate], current_nodes: &[Node]) -> Option<ReplacementCandidate> {
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
            let mut id_sorted_best_results = best_results.to_owned();
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
                    let nakamoto_score_before = NakamotoScore::new_from_nodes(&nodes_initial);
                    added_nodes.push((
                        best_result.node.clone(),
                        best_result.score.describe_difference_from(&nakamoto_score_before).1,
                    ));
                    available_nodes.retain(|n| n.id != best_result.node.id);
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
                                    best_result.node.id.to_string().split('-').next().unwrap_or_default(),
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
            added_nodes_desc: added_nodes,
            removed_nodes_desc: self.removed_nodes_desc,
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
                    let nakamoto_score_before = NakamotoScore::new_from_nodes(&self.nodes);
                    self.removed_nodes_desc.push((
                        best_result.node.clone(),
                        best_result.score.describe_difference_from(&nakamoto_score_before).1,
                    ));
                    self.nodes.retain(|n| n.id != best_result.node.id);
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
                                    best_result.node.id.to_string().split('-').next().unwrap_or_default(),
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
            added_nodes_desc: self.added_nodes_desc,
            removed_nodes_desc: self.removed_nodes_desc,
            comment,
            run_log,
        })
    }

    pub fn without_duplicate_added_removed(self) -> DecentralizedSubnet {
        let common_nodes: Vec<PrincipalId> = self
            .removed_nodes_desc
            .iter()
            .filter_map(|(node_removed, _)| {
                if self.added_nodes_desc.iter().any(|(node_added, _)| node_removed.id == node_added.id) {
                    Some(node_removed.id)
                } else {
                    None
                }
            })
            .collect();

        if !common_nodes.is_empty() {
            info!("Removing nodes which have been removed and then added back: {:?}", common_nodes);

            let added_nodes_desc = self
                .added_nodes_desc
                .into_iter()
                .filter(|(node_added, _)| !common_nodes.iter().any(|common_node| common_node == &node_added.id))
                .collect();

            let removed_nodes_desc = self
                .removed_nodes_desc
                .into_iter()
                .filter(|(node_removed, _)| !common_nodes.iter().any(|common_node| common_node == &node_removed.id))
                .collect();

            Self {
                id: self.id,
                nodes: self.nodes.clone(),
                added_nodes_desc,
                removed_nodes_desc,
                comment: self.comment.clone(),
                run_log: self.run_log.clone(),
            }
        } else {
            self
        }
    }

    fn _node_to_replacement_candidate(&self, subnet_nodes: &[Node], touched_node: &Node, err_log: &mut Vec<String>) -> Option<ReplacementCandidate> {
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
            added_nodes_desc: Vec::new(),
            removed_nodes_desc: Vec::new(),
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

pub trait AvailableNodesQuerier {
    fn available_nodes(&self) -> BoxFuture<'_, Result<Vec<Node>, NetworkError>>;
}

#[derive(Clone)]
pub enum SubnetQueryBy {
    SubnetId(PrincipalId),
    NodeList(Vec<Node>),
}

pub trait NodesConverter {
    fn get_nodes<'a>(&'a self, from: &'a [PrincipalId]) -> BoxFuture<'a, Result<Vec<Node>, NetworkError>>;
}

pub trait SubnetQuerier {
    fn subnet(&self, by: SubnetQueryBy) -> BoxFuture<'_, Result<DecentralizedSubnet, NetworkError>>;
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
        include_nodes: Vec<PrincipalId>,
        exclude_nodes: Vec<String>,
        only_nodes: Vec<String>,
        health_of_nodes: &'a IndexMap<PrincipalId, HealthStatus>,
        cordoned_features: Vec<NodeFeaturePair>,
    ) -> BoxFuture<'a, Result<SubnetChange, NetworkError>> {
        Box::pin(async move {
            SubnetChangeRequest {
                available_nodes: self.available_nodes().await?,
                ..Default::default()
            }
            .including_from_available(include_nodes.clone())
            .excluding_from_available(exclude_nodes.clone())
            .including_from_available(only_nodes.clone())
            .resize(size, 0, 0, health_of_nodes, cordoned_features)
        })
    }
}

pub trait Identifies<Node> {
    fn eq(&self, other: &Node) -> bool;
}

impl Identifies<Node> for PrincipalId {
    fn eq(&self, other: &Node) -> bool {
        &other.id == self
    }
}

impl Identifies<Node> for String {
    fn eq(&self, other: &Node) -> bool {
        other.matches_feature_value(self)
    }
}

impl Identifies<Node> for Node {
    fn eq(&self, other: &Node) -> bool {
        self == other
    }
}

trait MatchAnyNode<T: Identifies<Node>> {
    fn match_any(self, node: &Node) -> bool;
}

impl<T: Identifies<Node>> MatchAnyNode<T> for std::slice::Iter<'_, T> {
    fn match_any(mut self, node: &Node) -> bool {
        self.any(|n| n.eq(node))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NodeFeaturePair {
    pub feature: NodeFeature,
    pub value: String,
}

#[derive(Default, Clone, Debug)]
pub struct SubnetChangeRequest {
    subnet: DecentralizedSubnet,
    available_nodes: Vec<Node>,
    include_nodes: Vec<Node>,
    nodes_to_remove: Vec<Node>,
    nodes_to_keep: Vec<Node>,
}

impl SubnetChangeRequest {
    pub fn new(
        subnet: DecentralizedSubnet,
        available_nodes: Vec<Node>,
        include_nodes: Vec<Node>,
        nodes_to_remove: Vec<Node>,
        nodes_to_keep: Vec<Node>,
    ) -> Self {
        SubnetChangeRequest {
            subnet,
            available_nodes,
            include_nodes,
            nodes_to_remove,
            nodes_to_keep,
        }
    }

    pub fn keeping_from_used<T: Identifies<Node>>(self, nodes: Vec<T>) -> Self {
        let mut change_new = self.clone();
        let nodes_to_keep = self
            .subnet
            .nodes
            .into_iter()
            .filter(|node: &Node| nodes.iter().match_any(node))
            .collect_vec();
        change_new.nodes_to_keep.extend(nodes_to_keep);
        change_new
    }

    pub fn removing_from_used<T: Identifies<Node>>(self, nodes: Vec<T>) -> Self {
        let mut change_new = self.clone();
        let nodes_to_remove = self
            .subnet
            .nodes
            .into_iter()
            .filter(|node: &Node| nodes.iter().match_any(node))
            .collect_vec();
        change_new.nodes_to_remove.extend(nodes_to_remove);
        change_new
    }

    pub fn including_from_available<T: Identifies<Node>>(self, nodes: Vec<T>) -> Self {
        Self {
            include_nodes: self
                .available_nodes
                .iter()
                .filter(|node| nodes.iter().match_any(node))
                .cloned()
                .collect_vec(),
            ..self
        }
    }

    pub fn excluding_from_available<T: Identifies<Node>>(self, nodes: Vec<T>) -> Self {
        Self {
            available_nodes: self
                .available_nodes
                .iter()
                .filter(|node| !nodes.iter().match_any(node))
                .cloned()
                .collect_vec(),
            ..self
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

    /// Optimize is implemented by removing a certain number of nodes and then
    /// adding the same number back.
    pub fn optimize(
        mut self,
        optimize_count: usize,
        replacements_unhealthy_with_desc: &[(Node, String)],
        health_of_nodes: &IndexMap<PrincipalId, HealthStatus>,
        cordoned_features: Vec<NodeFeaturePair>,
    ) -> Result<SubnetChange, NetworkError> {
        let old_nodes = self.subnet.nodes.clone();
        self.subnet = self.subnet.without_nodes(replacements_unhealthy_with_desc.to_owned())?;
        let result = self.resize(
            optimize_count + replacements_unhealthy_with_desc.len(),
            optimize_count,
            replacements_unhealthy_with_desc.len(),
            health_of_nodes,
            cordoned_features,
        )?;
        Ok(SubnetChange { old_nodes, ..result })
    }

    pub fn rescue(
        mut self,
        health_of_nodes: &IndexMap<PrincipalId, HealthStatus>,
        cordoned_features: Vec<NodeFeaturePair>,
    ) -> Result<SubnetChange, NetworkError> {
        let old_nodes = self.subnet.nodes.clone();
        let nodes_to_remove = self
            .subnet
            .nodes
            .iter()
            .filter(|n| !self.nodes_to_keep.contains(n))
            .cloned()
            .collect_vec();
        self.subnet = self.subnet.without_nodes(
            nodes_to_remove
                .into_iter()
                .map(|n| (n, "Recovering unhealthy subnet".to_string()))
                .collect(),
        )?;

        info!("Nodes left in the subnet:\n{:#?}", &self.subnet.nodes);
        let result = self.resize(
            self.subnet.removed_nodes_desc.len(),
            0,
            self.subnet.removed_nodes_desc.len(),
            health_of_nodes,
            cordoned_features,
        )?;
        Ok(SubnetChange { old_nodes, ..result })
    }

    /// Add or remove nodes from the subnet.
    pub fn resize(
        &self,
        how_many_nodes_to_add: usize,
        how_many_nodes_to_remove: usize,
        how_many_nodes_unhealthy: usize,
        health_of_nodes: &IndexMap<PrincipalId, HealthStatus>,
        cordoned_features: Vec<NodeFeaturePair>,
    ) -> Result<SubnetChange, NetworkError> {
        let old_nodes = self.subnet.nodes.clone();

        let all_healthy_nodes = self
            .available_nodes
            .clone()
            .into_iter()
            .filter(|n| !self.include_nodes.contains(n))
            .filter(|n| health_of_nodes.get(&n.id).unwrap_or(&HealthStatus::Unknown) == &HealthStatus::Healthy)
            .collect::<Vec<_>>();

        let available_nodes = all_healthy_nodes
            .into_iter()
            .filter(|n| {
                for cordoned_feature in &cordoned_features {
                    if let Some(node_feature) = n.features.get(&cordoned_feature.feature) {
                        if PartialEq::eq(&node_feature, &cordoned_feature.value) {
                            // Node contains cordoned feature
                            // exclude it from available pool
                            return false;
                        }
                    }
                }
                // Node doesn't contain any cordoned features
                // include it the available pool
                true
            })
            .collect_vec();

        info!(
            "Resizing subnet {} by adding {} and removing {} (from which {} unhealthy) nodes. Total available {} healthy nodes.",
            self.subnet.id,
            how_many_nodes_to_add,
            how_many_nodes_to_remove,
            how_many_nodes_unhealthy,
            available_nodes.len(),
        );

        let resized_subnet = if how_many_nodes_to_remove > 0 {
            self.subnet
                .clone()
                .subnet_with_fewer_nodes(how_many_nodes_to_remove)
                .map_err(|e| NetworkError::ResizeFailed(e.to_string()))?
        } else {
            self.subnet.clone()
        };

        let available_nodes = available_nodes
            .iter()
            .cloned()
            .chain(resized_subnet.removed_nodes_desc.iter().map(|(n, _)| n.clone()))
            .filter(|n| health_of_nodes.get(&n.id).unwrap_or(&HealthStatus::Unknown) == &HealthStatus::Healthy)
            .filter(|n| {
                for cordoned_feature in &cordoned_features {
                    if let Some(node_feature) = n.features.get(&cordoned_feature.feature) {
                        if PartialEq::eq(&node_feature, &cordoned_feature.value) {
                            // Node contains cordoned feature
                            // exclude it from available pool
                            return false;
                        }
                    }
                }
                // Node doesn't contain any cordoned features
                // include it the available pool
                true
            })
            .collect::<Vec<_>>();
        let resized_subnet = resized_subnet
            .with_nodes(
                self.include_nodes
                    .iter()
                    .map(|n| (n.clone(), "included as per user request".to_string()))
                    .collect(),
            )
            .subnet_with_more_nodes(how_many_nodes_to_add, &available_nodes)
            .map_err(|e| NetworkError::ResizeFailed(e.to_string()))?
            .without_duplicate_added_removed();

        let penalties_after_change = DecentralizedSubnet::check_business_rules_for_subnet_with_nodes(&self.subnet.id, &resized_subnet.nodes)
            .map_err(|e| NetworkError::ResizeFailed(e.to_string()))?
            .0;

        let subnet_change = SubnetChange {
            id: self.subnet.id,
            old_nodes,
            new_nodes: resized_subnet.nodes,
            removed_nodes_desc: resized_subnet.removed_nodes_desc,
            added_nodes_desc: resized_subnet.added_nodes_desc,
            penalties_after_change,
            comment: resized_subnet.comment,
            run_log: resized_subnet.run_log,
        };
        Ok(subnet_change)
    }

    /// Evaluates the subnet change request to simulate the requested topology
    /// change. Command returns all the information about the subnet before
    /// and after the change.
    pub fn evaluate(
        self,
        health_of_nodes: &IndexMap<PrincipalId, HealthStatus>,
        cordoned_features: Vec<NodeFeaturePair>,
    ) -> Result<SubnetChange, NetworkError> {
        self.resize(0, 0, 0, health_of_nodes, cordoned_features)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SubnetChange {
    pub id: PrincipalId,
    pub old_nodes: Vec<Node>,
    pub new_nodes: Vec<Node>,
    pub removed_nodes_desc: Vec<(Node, String)>,
    pub added_nodes_desc: Vec<(Node, String)>,
    pub penalties_after_change: usize,
    pub comment: Option<String>,
    pub run_log: Vec<String>,
}

impl SubnetChange {
    pub fn with_nodes(self, nodes_to_add_with_desc: Vec<(Node, String)>) -> Self {
        let nodes_to_add: AHashSet<_> = nodes_to_add_with_desc.iter().map(|(n, _)| n).collect();
        let new_nodes = [self.new_nodes, nodes_to_add.into_iter().cloned().collect_vec()].concat();
        let penalties_after_change = DecentralizedSubnet::check_business_rules_for_subnet_with_nodes(&self.id, &new_nodes)
            .expect("Business rules check should succeed")
            .0;
        Self {
            new_nodes,
            added_nodes_desc: nodes_to_add_with_desc,
            penalties_after_change,
            ..self
        }
    }

    pub fn without_nodes(mut self, nodes_to_remove_with_desc: Vec<(Node, String)>) -> Self {
        let nodes_to_rm: AHashSet<_> = nodes_to_remove_with_desc.iter().map(|(n, _)| n).collect();
        self.removed_nodes_desc.extend(nodes_to_remove_with_desc.clone());
        self.new_nodes.retain(|n| !nodes_to_rm.contains(n));
        self.penalties_after_change = DecentralizedSubnet::check_business_rules_for_subnet_with_nodes(&self.id, &self.new_nodes)
            .expect("Business rules check should succeed")
            .0;
        self
    }

    pub fn added(&self) -> Vec<(Node, String)> {
        self.added_nodes_desc.clone()
    }

    pub fn removed(&self) -> Vec<(Node, String)> {
        self.removed_nodes_desc.clone()
    }

    pub fn before(&self) -> DecentralizedSubnet {
        DecentralizedSubnet {
            id: self.id,
            nodes: self.old_nodes.clone(),
            added_nodes_desc: Vec::new(),
            removed_nodes_desc: Vec::new(),
            comment: self.comment.clone(),
            run_log: Vec::new(),
        }
    }

    pub fn after(&self) -> DecentralizedSubnet {
        DecentralizedSubnet {
            id: self.id,
            nodes: self.new_nodes.clone(),
            added_nodes_desc: self.added_nodes_desc.clone(),
            removed_nodes_desc: self.removed_nodes_desc.clone(),
            comment: self.comment.clone(),
            run_log: self.run_log.clone(),
        }
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
    pub subnets: IndexMap<PrincipalId, ic_management_types::Subnet>,
}

impl NetworkHealRequest {
    pub fn new(subnets: IndexMap<PrincipalId, ic_management_types::Subnet>) -> Self {
        Self { subnets }
    }

    pub async fn heal_and_optimize(
        &self,
        mut available_nodes: Vec<Node>,
        health_of_nodes: &IndexMap<PrincipalId, HealthStatus>,
        cordoned_features: Vec<NodeFeaturePair>,
    ) -> Result<Vec<SubnetChangeResponse>, NetworkError> {
        let mut subnets_changed = Vec::new();
        let subnets_to_heal = unhealthy_with_nodes(&self.subnets, health_of_nodes)
            .iter()
            .flat_map(|(subnet_id, unhealthy_nodes)| {
                let unhealthy_nodes = unhealthy_nodes.iter().map(Node::from).collect::<Vec<_>>();
                let unhealthy_subnet = self.subnets.get(subnet_id).ok_or(NetworkError::SubnetNotFound(*subnet_id))?;

                Ok::<NetworkHealSubnets, NetworkError>(NetworkHealSubnets {
                    name: unhealthy_subnet.metadata.name.clone(),
                    decentralized_subnet: DecentralizedSubnet::from(unhealthy_subnet),
                    unhealthy_nodes,
                })
            })
            .sorted_by(|a, b| a.cmp(b).reverse())
            .collect_vec();

        if subnets_to_heal.is_empty() {
            info!("Nothing to do! All subnets are healthy.")
        }

        for subnet in subnets_to_heal {
            // If more than 1/3 nodes do not have the latest subnet state, subnet will stall.
            // From those 1/2 are added and 1/2 removed -> nodes_in_subnet/3 * 1/2 = nodes_in_subnet/6
            let max_replaceable_nodes = subnet.decentralized_subnet.nodes.len() / 6;

            let unhealthy_nodes = if subnet.unhealthy_nodes.len() > max_replaceable_nodes {
                let unhealthy_nodes = subnet.unhealthy_nodes.into_iter().take(max_replaceable_nodes).collect_vec();
                warn!(
                    "Subnet {}: replacing {} of {} unhealthy nodes: {:?}",
                    subnet.decentralized_subnet.id,
                    max_replaceable_nodes,
                    unhealthy_nodes.len(),
                    unhealthy_nodes.iter().map(|node| node.id).collect_vec()
                );
                unhealthy_nodes
            } else {
                info!(
                    "Subnet {}: replacing {} unhealthy nodes: {:?}, and optimizing {} nodes. Max safely replaceable nodes based on subnet size: {}",
                    subnet.decentralized_subnet.id,
                    subnet.unhealthy_nodes.len(),
                    subnet
                        .unhealthy_nodes
                        .iter()
                        .map(|node| node.id.to_string().split('-').next().unwrap().to_string())
                        .collect_vec(),
                    max_replaceable_nodes - subnet.unhealthy_nodes.len(),
                    max_replaceable_nodes
                );
                subnet.unhealthy_nodes
            };
            let unhealthy_nodes_len = unhealthy_nodes.len();
            let optimize_limit = max_replaceable_nodes - unhealthy_nodes_len;
            let change_req = SubnetChangeRequest {
                subnet: subnet.decentralized_subnet.clone(),
                available_nodes: available_nodes.clone(),
                ..Default::default()
            };

            let unhealthy_nodes_with_desc = &generate_removed_nodes_description(&subnet.decentralized_subnet.nodes, &unhealthy_nodes);
            info!(
                "Subnet {}: unhealthy nodes: {:?}",
                subnet.decentralized_subnet.id,
                unhealthy_nodes_with_desc
                    .iter()
                    .map(|(node, desc)| format!("{} --> {}", node.id, desc))
                    .collect_vec()
            );

            // Try to replace from 0 to optimize_limit nodes to optimize the network,
            // and choose the replacement of the fewest nodes that gives the most decentralization benefit.
            let changes = (0..=optimize_limit)
                .filter_map(|num_nodes_to_optimize| {
                    change_req
                        .clone()
                        .optimize(
                            num_nodes_to_optimize,
                            unhealthy_nodes_with_desc,
                            health_of_nodes,
                            cordoned_features.clone(),
                        )
                        .map_err(|e| warn!("{}", e))
                        .ok()
                })
                .map(|change| SubnetChangeResponse::from(&change).with_health_of_nodes(health_of_nodes.clone()))
                .collect::<Vec<_>>();

            if changes.is_empty() {
                warn!("No suitable changes found for subnet {}", subnet.decentralized_subnet.id);
                continue;
            }
            for change in &changes {
                info!(
                    "Replacing {} nodes in subnet {} results in subnet with business-rules penalty {} and Nakamoto coefficient: {}\n",
                    change.removed_with_desc.len(),
                    subnet.decentralized_subnet.id,
                    change.penalties_after_change,
                    change.score_after
                );
            }

            // Some community members have expressed concern about the business-rules penalty.
            // https://forum.dfinity.org/t/subnet-management-tdb26-nns/33663/26 and a few comments below.
            // As a compromise, we will choose the change that has the lowest business-rules penalty,
            // or if there is no improvement in the business-rules penalty, we will choose the change
            // that replaces the fewest nodes.
            let penalty_optimize_min = changes.iter().map(|change| change.penalties_after_change).min().unwrap();
            info!("Min business-rules penalty: {}", penalty_optimize_min);

            let changes = changes
                .into_iter()
                .filter(|change| change.penalties_after_change == penalty_optimize_min)
                .collect::<Vec<_>>();

            let changes_max_score = changes
                .iter()
                .max_by_key(|change| change.score_after.clone())
                .expect("Failed to find a replacement with the highest Nakamoto coefficient");

            let optimizations_desc = changes
                .iter()
                .enumerate()
                .skip(1)
                .map(|(num_opt, change)| {
                    format!(
                        "- {} additional node{}{}: {}",
                        num_opt,
                        if num_opt > 1 { "s" } else { "" },
                        if change.penalties_after_change > 0 {
                            format!(" (solution penalty: {})", change.penalties_after_change)
                        } else {
                            "".to_string()
                        },
                        change
                            .score_after
                            .describe_difference_from(&changes[num_opt.saturating_sub(1)].score_after)
                            .1
                    )
                })
                .collect::<Vec<_>>();
            info!("Max score: {}", changes_max_score.score_after);

            let change = if penalty_optimize_min > 0 && penalty_optimize_min == changes[0].penalties_after_change {
                info!("No reduction in business-rules penalty, choosing the first change");
                &changes[0]
            } else {
                changes
                    .iter()
                    .find(|change: &&SubnetChangeResponse| change.score_after == changes_max_score.score_after)
                    .expect("No suitable changes found")
            };

            info!(
                "Replacing {} nodes in subnet {} gives Nakamoto coefficient: {}\n",
                change.removed_with_desc.len(),
                subnet.decentralized_subnet.id,
                change.score_after
            );

            let num_opt = change.removed_with_desc.len() - unhealthy_nodes_len;
            let reason_additional_optimizations = if num_opt == 0 {
                format!(
                    "

Calculated impact on subnet decentralization if replacing:

{}

Based on the calculated impact, not replacing additional nodes to improve optimization.
",
                    optimizations_desc.join("\n")
                )
            } else {
                format!("

Calculated impact on subnet decentralization if replacing:

{}

Based on the calculated impact, replacing {} additional nodes to improve optimization

Note: the heuristic for node replacement relies not only on the Nakamoto coefficient but also on other factors that iteratively optimize network topology.
Due to this, Nakamoto coefficients may not directly increase in every node replacement proposal.
Code for comparing decentralization of two candidate subnet topologies is at:
https://github.com/dfinity/dre/blob/79066127f58c852eaf4adda11610e815a426878c/rs/decentralization/src/nakamoto/mod.rs#L342
",
                    optimizations_desc.join("\n"),
                    num_opt
                )
            };

            let mut motivations: Vec<String> = Vec::new();

            for node in unhealthy_nodes.iter() {
                motivations.push(format!(
                    "replacing {} node {}",
                    health_of_nodes
                        .get(&node.id)
                        .map(|s| s.to_string().to_lowercase())
                        .unwrap_or("unhealthy".to_string()),
                    node.id
                ));
            }

            let unhealthy_nodes_ids = unhealthy_nodes.iter().map(|node| node.id).collect::<HashSet<_>>();
            for (node, _desc) in change.removed_with_desc.iter().filter(|(n, _)| !unhealthy_nodes_ids.contains(n)) {
                motivations.push(format!("replacing node {} to optimize network topology", node));
            }

            let nodes_added = change.added_with_desc.iter().map(|(node_id, _)| node_id).collect::<HashSet<_>>();
            available_nodes.retain(|node| !nodes_added.contains(&node.id));

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

pub fn generate_removed_nodes_description(subnet_nodes: &[Node], remove_nodes: &[Node]) -> Vec<(Node, String)> {
    let mut subnet_nodes: AHashMap<PrincipalId, Node> = AHashMap::from_iter(subnet_nodes.iter().map(|n| (n.id, n.clone())));
    let mut result = Vec::new();
    for node in remove_nodes {
        let nakamoto_before = NakamotoScore::new_from_nodes(subnet_nodes.values());
        subnet_nodes.remove(&node.id);
        let nakamoto_after = NakamotoScore::new_from_nodes(subnet_nodes.values());
        let nakamoto_diff = nakamoto_after.describe_difference_from(&nakamoto_before).1;

        result.push((node.clone(), nakamoto_diff));
    }
    result
}

pub fn generate_added_node_description(subnet_nodes: &[Node], add_nodes: &[Node]) -> Vec<(Node, String)> {
    let mut subnet_nodes: AHashMap<PrincipalId, Node> = AHashMap::from_iter(subnet_nodes.iter().map(|n| (n.id, n.clone())));
    let mut result = Vec::new();
    for node in add_nodes {
        let nakamoto_before = NakamotoScore::new_from_nodes(subnet_nodes.values());
        subnet_nodes.insert(node.id, node.clone());
        let nakamoto_after = NakamotoScore::new_from_nodes(subnet_nodes.values());
        let nakamoto_diff = nakamoto_after.describe_difference_from(&nakamoto_before).1;

        result.push((node.clone(), nakamoto_diff));
    }
    result
}
