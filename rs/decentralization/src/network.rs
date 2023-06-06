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

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct DataCenterInfo {
    city: String,
    country: String,
    continent: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Node {
    pub id: PrincipalId,
    pub features: nakamoto::NodeFeatures,
    pub dfinity_owned: bool,
    pub decentralized: bool,
}

impl Node {
    pub fn new_test_node(
        node_number: u64,
        features: nakamoto::NodeFeatures,
        dfinity_owned: bool,
        decentralized: bool,
    ) -> Self {
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

    pub fn matches_feature_value(&self, value: &String) -> bool {
        self.id.to_string() == *value || self.get_features().feature_map.values().any(|v| *v == *value)
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
            features: nakamoto::NodeFeatures::from_iter(
                [
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
                ]
                .into_iter(),
            ),
            dfinity_owned: n.dfinity_owned.unwrap_or_default(),
            decentralized: n.decentralized,
        }
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct DecentralizedSubnet {
    pub id: PrincipalId,
    pub nodes: Vec<Node>,
    pub removed_nodes: Vec<Node>,
    pub min_nakamoto_coefficients: Option<MinNakamotoCoefficients>,
    pub comment: Option<String>,
    pub run_log: Vec<String>,
}

impl DecentralizedSubnet {
    /// Return a new instance of a DecentralizedSubnet that does not contain the
    /// provided nodes.
    pub fn without_nodes(&self, nodes: &[PrincipalId]) -> Result<Self, NetworkError> {
        let mut new_subnet_nodes = self.nodes.clone();
        let mut removed = Vec::new();
        for node in nodes {
            if let Some(index) = new_subnet_nodes.iter().position(|n| n.id == *node) {
                removed.push(new_subnet_nodes.remove(index));
            } else {
                return Err(NetworkError::NodeNotFound(*node));
            }
        }
        Ok(Self {
            id: self.id,
            nodes: new_subnet_nodes,
            removed_nodes: removed.clone(),
            min_nakamoto_coefficients: self.min_nakamoto_coefficients.clone(),
            comment: self.comment.clone(),
            run_log: {
                let mut run_log = self.run_log.clone();
                run_log.push(format!("Without nodes {:?}", removed.iter().map(|n| n.id)));
                run_log
            },
        })
    }

    /// Return a new instance of a DecentralizedSubnet that contains the
    /// provided nodes.
    pub fn with_nodes(&self, nodes: Vec<Node>) -> Self {
        Self {
            id: self.id,
            nodes: self.nodes.clone().into_iter().chain(nodes.clone()).collect(),
            removed_nodes: self.removed_nodes.clone(),
            min_nakamoto_coefficients: self.min_nakamoto_coefficients.clone(),
            comment: self.comment.clone(),
            run_log: {
                let mut run_log = self.run_log.clone();
                run_log.push(format!("With nodes {:?}", nodes.iter().map(|n| n.id)));
                run_log
            },
        }
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

        let dfinity_owned_nodes_count: usize = nodes.iter().map(|n| n.dfinity_owned as usize).sum();
        let target_dfinity_owned_nodes_count =
            if subnet_id_str == *"tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe" {
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
            checks.push(format!(
                "Subnet has {} non-decentralized node(s)",
                count_non_decentralized_nodes
            ));
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

        match nakamoto_scores.score_feature(&NodeFeature::NodeProvider) {
            Some(score) => {
                if score <= 1.0 && nodes.len() > 3 {
                    // We restrict to subnets with >3 nodes to be able to build subnet from scratch
                    return Err(anyhow::anyhow!("A single Node Provider can halt a subnet"));
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
            match (
                nakamoto_scores.score_feature(feature),
                nakamoto_scores.controlled_nodes(feature),
            ) {
                (Some(score), Some(controlled_nodes)) => {
                    if score == 1.0 && controlled_nodes > nodes.len() * 2 / 3 {
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

        debug!(
            "Business rules checks succeeded for subnet {}: {:?}",
            subnet_id.to_string(),
            checks
        );
        Ok((penalties, checks))
    }

    fn _calc_nakamoto_score(nodes: &[Node]) -> NakamotoScore {
        NakamotoScore::new_from_nodes(nodes)
    }

    /// Calculate and return the NakamotoScore for the nodes in the subnet
    pub fn nakamoto_score(&self) -> NakamotoScore {
        Self::_calc_nakamoto_score(&self.nodes)
    }

    pub fn new_extended_subnet(
        &self,
        num_nodes_to_add: usize,
        available_nodes: &[Node],
    ) -> anyhow::Result<DecentralizedSubnet> {
        let mut run_log = self.run_log.clone();

        let mut nodes_initial = self.nodes.clone();
        let mut available_nodes = available_nodes.to_vec();
        let orig_available_nodes_len = &available_nodes.len();
        let mut nodes_after_extension = self.nodes.clone();
        let mut comment = None;
        let mut total_penalty = 0;
        let mut business_rules_log: Vec<String> = Vec::new();

        let line = format!("Nakamoto score before extension {}", self.nakamoto_score());
        info!("{}", &line);
        run_log.push(line);

        struct SortResult {
            index: usize,
            node: Node,
            score: NakamotoScore,
            penalty: usize,
            business_rules_log: Vec<String>,
        }

        for i in 0..num_nodes_to_add {
            run_log.push("***********************************************************".to_string());
            run_log.push(format!("***  Adding node {}/{}", i + 1, num_nodes_to_add));
            run_log.push("***********************************************************".to_string());

            let sorted_good_nodes: Vec<SortResult> = available_nodes
                .iter()
                .enumerate()
                .filter_map(|(index, node)| {
                    let candidate_subnet_nodes: Vec<Node> = nodes_initial.iter().chain([node]).cloned().collect();
                    match Self::_check_business_rules_for_nodes(
                        &self.id,
                        &candidate_subnet_nodes,
                        &self.min_nakamoto_coefficients,
                    ) {
                        Ok((penalty, business_rules_log)) => {
                            let new_score = Self::_calc_nakamoto_score(&candidate_subnet_nodes);

                            let line = format!(
                                "Picked one extension node {} and got Nakamoto score {} and penalty {}",
                                node.id, new_score, penalty
                            );
                            debug!("{}", &line);
                            run_log.push(line);

                            Some(SortResult {
                                index,
                                node: node.clone(),
                                score: new_score,
                                penalty,
                                business_rules_log,
                            })
                        }
                        Err(err) => {
                            let line = format!(
                                "Extension candidate node {} not suitable due to failed business rule {}",
                                node.id, err
                            );
                            debug!("{}", &line);
                            run_log.push(line);
                            None
                        }
                    }
                })
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
                .collect();

            run_log.push("Sorted candidate nodes, with the best candidate at the end:".to_string());
            run_log.push(
                "     <node-id>                                                      <penalty>  <Nakamoto score>"
                    .to_string(),
            );
            for s in &sorted_good_nodes {
                run_log.push(format!(" -=> {} {} {}", s.node.id, s.penalty, s.score));
            }

            let first_best_result = sorted_good_nodes.iter().last();
            let mut best_results = vec![];
            if let Some(result) = first_best_result {
                for candidate in sorted_good_nodes.iter().rev() {
                    // To filter the best results, we must take the penalty
                    // applied to the subnet as well.  If not, even if two
                    // candidates have the same score, we could end up with a
                    // higher penalty in the resulting subnet as we choose
                    // randomly one of the best candidates in those results We
                    // know from the previous sorting that the last element in
                    // the array of results will have the lowest penalty and
                    // nakamoto score, so we can compare against this one.
                    if candidate.score == result.score && candidate.penalty <= result.penalty {
                        best_results.push(candidate)
                    } else {
                        break;
                    }
                }
            }

            /// Deterministically choose a result in the list based on the list
            /// of current nodes.  Since the node IDs are unique, we seed a PRNG
            /// with the sorted joined node IDs. We then choose a result
            /// randomly but deterministically using this seed.
            ///
            /// NOTE: This funnction cannot be moved outside of this context
            /// because SortResult is defined in this context
            fn choose_deterministic_random<'a>(
                best_results: Vec<&'a SortResult>,
                current_nodes: &[Node],
            ) -> Option<&'a SortResult> {
                if best_results.is_empty() {
                    None
                } else {
                    // We sort the current nodes by alphabetical order on their
                    // PrincipalIDs to ensure consistency of the seed with the
                    // same machines in the subnet
                    let mut id_sorted_current_nodes = current_nodes.to_owned();
                    id_sorted_current_nodes
                        .sort_by(|n1, n2| std::cmp::Ord::cmp(&n1.id.to_string(), &n2.id.to_string()));
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
                    id_sorted_best_results
                        .sort_by(|r1, r2| std::cmp::Ord::cmp(&r1.node.id.to_string(), &r2.node.id.to_string()));
                    id_sorted_best_results.choose(&mut rng).cloned()
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
            let best_result = choose_deterministic_random(best_results, &self.nodes);

            match best_result {
                Some(best_result) => {
                    let line = format!("Nakamoto score after extension {}", best_result.score);
                    info!("{}", &line);
                    run_log.push(line);
                    available_nodes.swap_remove(best_result.index);
                    nodes_after_extension.push(best_result.node.clone());
                    nodes_initial.push(best_result.node.clone());
                    total_penalty += best_result.penalty;
                    business_rules_log.extend(
                        best_result
                            .business_rules_log
                            .iter()
                            .map(|s| format!("node {}/{} ({}): {}", i + 1, num_nodes_to_add, best_result.node.id, s))
                            .collect::<Vec<String>>(),
                    );
                    if i + 1 == num_nodes_to_add {
                        if total_penalty != 0 {
                            comment = Some(format!(
                                "Subnet extension with {} nodes finished with the total penalty {}. Penalty causes throughout the extension:\n{}\n\n{}",
                                num_nodes_to_add,
                                total_penalty,
                                business_rules_log.join("\n"),
                                if num_nodes_to_add > 1 {
                                    "Note that the penalty for nodes before the last node may not be relevant after all extensions are completed. We leave this to humans to assess."
                                } else { "" }
                            ));
                        } else {
                            comment = None;
                        }
                    }
                }
                None => {
                    return Err(anyhow!(
                        "Could not complete the extension. Run log:\n{}",
                        run_log.join("\n")
                    ))
                }
            }
        }
        assert_eq!(nodes_after_extension.len(), self.nodes.len() + num_nodes_to_add);
        assert_eq!(orig_available_nodes_len - available_nodes.len(), num_nodes_to_add);

        Ok(Self {
            id: self.id,
            nodes: nodes_after_extension,
            removed_nodes: self.removed_nodes.clone(),
            min_nakamoto_coefficients: self.min_nakamoto_coefficients.clone(),
            comment,
            run_log,
        })
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

#[async_trait]
pub trait SubnetQuerier {
    async fn subnet(&self, id: &PrincipalId) -> Result<DecentralizedSubnet, NetworkError>;
    async fn subnet_of_nodes(&self, nodes: &[PrincipalId]) -> Result<DecentralizedSubnet, NetworkError>;
}

#[derive(Clone, Serialize, Deserialize, Debug, strum_macros::Display)]
pub enum DecentralizationError {
    FeatureNotAvailable,
}

impl ResponseError for DecentralizationError {
    fn error_response(&self) -> HttpResponse {
        let out: serde_json::Value =
            serde_json::from_str("{\"message\": \"NodeFeature not available. For access contact the administrator\"}")
                .unwrap();
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
    async fn modify_subnet_nodes(&self, subnet_id: PrincipalId) -> Result<SubnetChangeRequest, NetworkError> {
        Ok(SubnetChangeRequest {
            available_nodes: self.available_nodes().await?,
            subnet: self.subnet(&subnet_id).await?,
            ..Default::default()
        })
    }

    async fn replace_subnet_nodes(&self, nodes: &[PrincipalId]) -> Result<SubnetChangeRequest, NetworkError> {
        SubnetChangeRequest {
            available_nodes: self.available_nodes().await?,
            subnet: self.subnet_of_nodes(nodes).await?,
            ..Default::default()
        }
        .remove(nodes)
    }

    async fn create_subnet(
        &self,
        size: usize,
        min_nakamoto_coefficients: Option<MinNakamotoCoefficients>,
    ) -> Result<SubnetChange, NetworkError> {
        SubnetChangeRequest {
            available_nodes: self.available_nodes().await?,
            min_nakamoto_coefficients,
            ..Default::default()
        }
        .resize(size, 0)
    }
}

#[derive(Default, Clone)]
pub struct SubnetChangeRequest {
    subnet: DecentralizedSubnet,
    available_nodes: Vec<Node>,
    include_nodes: Vec<PrincipalId>,
    removed_nodes: Vec<Node>,
    improve_count: usize,
    min_nakamoto_coefficients: Option<MinNakamotoCoefficients>,
}

impl SubnetChangeRequest {
    pub fn new(
        subnet: DecentralizedSubnet,
        available_nodes: Vec<Node>,
        include_nodes: Vec<PrincipalId>,
        removed_nodes: Vec<Node>,
        improve_count: usize,
        min_nakamoto_coefficients: Option<MinNakamotoCoefficients>,
    ) -> Self {
        SubnetChangeRequest {
            subnet,
            available_nodes,
            include_nodes,
            removed_nodes,
            improve_count,
            min_nakamoto_coefficients,
        }
    }

    pub fn subnet(&self) -> DecentralizedSubnet {
        self.subnet.clone()
    }

    pub fn include_nodes(self, nodes: Vec<PrincipalId>) -> Self {
        Self {
            include_nodes: self.include_nodes.into_iter().chain(nodes).collect(),
            ..self
        }
    }

    pub fn exclude_nodes(self, exclude_nodes_or_features: Vec<String>) -> Self {
        Self {
            available_nodes: self
                .available_nodes
                .into_iter()
                .filter(|n| !exclude_nodes_or_features.iter().any(|v| n.matches_feature_value(v)))
                .collect(),
            ..self
        }
    }

    pub fn with_custom_available_nodes(self, nodes: Vec<Node>) -> Self {
        Self {
            available_nodes: nodes,
            ..self
        }
    }

    pub fn with_min_nakamoto_coefficients(self, min_nakamoto_coefficients: Option<MinNakamotoCoefficients>) -> Self {
        Self {
            min_nakamoto_coefficients,
            ..self
        }
    }

    /// Remove nodes from the subnet such that we keep nodes with the best
    /// Nakamoto coefficient.
    pub fn remove_nodes_and_keep_best_nakamoto(
        &self,
        how_many_nodes: usize,
    ) -> Result<DecentralizedSubnet, NetworkError> {
        let mut subnet = self.subnet.clone();

        subnet.run_log.push(format!(
            "Nakamoto score before removing nodes {}",
            NakamotoScore::new_from_nodes(subnet.nodes.as_slice())
        ));
        for i in 0..how_many_nodes {
            // Find the node that gives the best score when removed.
            let (j, score_max) = subnet
                .clone()
                .nodes
                .iter()
                .enumerate()
                .map(|(j, _node)| {
                    let mut subnet = subnet.clone();
                    subnet.nodes.swap_remove(j);
                    (j, NakamotoScore::new_from_nodes(subnet.nodes.as_slice()))
                })
                .max_by_key(|(_, score)| score.clone())
                .ok_or_else(|| {
                    NetworkError::ResizeFailed(format!(
                        "Cannot remove {} nodes from subnet with {} nodes",
                        how_many_nodes,
                        self.subnet.nodes.len()
                    ))
                })?;

            // Remove the node from the subnet.
            let node_removed = subnet.nodes.swap_remove(j);
            subnet.run_log.push(format!(
                "Removed {}/{} node {} with score {}",
                i + 1,
                how_many_nodes,
                node_removed.id,
                score_max
            ));
        }
        subnet.run_log.push(format!(
            "Nakamoto score after removing nodes {}",
            NakamotoScore::new_from_nodes(subnet.nodes.as_slice())
        ));
        Ok(subnet)
    }

    /// Add or remove nodes to the subnet.
    pub fn resize(
        &self,
        how_many_nodes_to_add: usize,
        how_many_nodes_to_remove: usize,
    ) -> Result<SubnetChange, NetworkError> {
        let included_nodes = self
            .available_nodes
            .iter()
            .filter(|n| self.include_nodes.contains(&n.id))
            .cloned()
            .collect::<Vec<_>>();

        let available_nodes = self
            .available_nodes
            .clone()
            .into_iter()
            .filter(|n| !included_nodes.contains(n))
            .collect::<Vec<_>>();

        let resized_subnet = if how_many_nodes_to_remove > 0 {
            self.remove_nodes_and_keep_best_nakamoto(how_many_nodes_to_remove)
                .map_err(|e| NetworkError::ResizeFailed(e.to_string()))?
        } else {
            self.subnet.clone()
        };
        let resized_subnet = resized_subnet
            .with_nodes(included_nodes)
            .with_min_nakamoto_coefficients(&self.min_nakamoto_coefficients)
            .new_extended_subnet(how_many_nodes_to_add, &available_nodes)
            .map_err(|e| NetworkError::ResizeFailed(e.to_string()))?;

        let subnet_change = SubnetChange {
            id: self.subnet.id,
            old_nodes: self
                .subnet
                .nodes
                .clone()
                .into_iter()
                .chain(self.removed_nodes.clone())
                .collect(),
            new_nodes: resized_subnet.nodes,
            min_nakamoto_coefficients: self.min_nakamoto_coefficients.clone(),
            comment: resized_subnet.comment,
            run_log: resized_subnet.run_log,
        };
        info!(
            "Subnet {} resized, {} nodes added, {} nodes removed",
            self.subnet.id, how_many_nodes_to_add, how_many_nodes_to_remove
        );
        Ok(subnet_change)
    }

    pub fn replace(self, nodes: &[PrincipalId]) -> Result<SubnetChange, NetworkError> {
        let subnet = self.subnet.without_nodes(nodes)?;
        let num_removed_nodes = subnet.removed_nodes.len();

        Self {
            subnet: subnet.clone(),
            ..self
        }
        .resize(num_removed_nodes, 0)
        .map(|mut sc| {
            sc.old_nodes.append(&mut subnet.removed_nodes.clone());
            sc
        })
    }

    pub fn optimize(self, max_replacements: usize) -> Result<SubnetChange, NetworkError> {
        let mut change_request = self.clone();

        let non_decentralized_nodes: Vec<Node> = self
            .subnet
            .nodes
            .iter()
            .filter(|n| !(n.decentralized))
            .cloned()
            .collect();
        let max_replacements = if !non_decentralized_nodes.is_empty() {
            let non_decentralized_node_ids = non_decentralized_nodes.iter().map(|n| n.id).collect::<Vec<_>>();
            let change = self.clone();
            let change = change
                .replace(&non_decentralized_node_ids)
                .expect("Replacing non-decentralized nodes should always work");
            if change.removed().len() >= max_replacements {
                info!(
                    "Replacing only non-decentralized nodes: {:?}",
                    &non_decentralized_node_ids
                );
                return Ok(change);
            };
            change_request = SubnetChangeRequest {
                subnet: DecentralizedSubnet {
                    nodes: change.new_nodes.clone(),
                    ..change_request.subnet
                },
                available_nodes: change_request
                    .available_nodes
                    .iter()
                    .filter(|n| !change.removed().contains(n))
                    .cloned()
                    .collect(),
                removed_nodes: change.removed(),
                ..change_request
            };
            max_replacements - change.removed().len()
        } else {
            max_replacements
        };

        info!("Optimizing {} nodes", max_replacements);

        let max_replacements = if max_replacements > 2 {
            warn!("Limiting the max replacements to 2 to prevent DOS");
            2
        } else {
            max_replacements
        };

        let results = self.subnet.nodes.iter().combinations(max_replacements).map(|nodes| {
            let mut change = change_request.clone();
            change
                .available_nodes
                .append(&mut nodes.iter().map(|n| (*n).clone()).collect::<Vec<_>>());
            change.replace(nodes.iter().map(|n| n.id).collect::<Vec<_>>().as_slice())
        });

        let errs = results.clone().map(|r| format!("{:?}", r)).collect::<Vec<_>>();

        match &results.clone().filter_map(|r| r.ok()).max_by(|sc1, sc2| {
            let score1 = NakamotoScore::new_from_nodes(&sc1.new_nodes);
            let score2 = NakamotoScore::new_from_nodes(&sc2.new_nodes);
            score1.cmp(&score2)
        }) {
            Some(best_result) => Ok(best_result.clone()),
            None => Err(NetworkError::ResizeFailed(format!(
                "Optimize failed, could not find any suitable solution for the request\n{}",
                errs.join("\n")
            ))),
        }
    }

    pub fn remove(self, nodes: &[PrincipalId]) -> Result<SubnetChangeRequest, NetworkError> {
        let subnet = self.subnet.without_nodes(nodes)?;
        Ok(SubnetChangeRequest {
            subnet: subnet.clone(),
            removed_nodes: self.removed_nodes.into_iter().chain(subnet.removed_nodes).collect(),
            ..self
        })
    }

    pub fn improve(self, count: usize) -> SubnetChangeRequest {
        Self {
            improve_count: count,
            ..self
        }
    }

    /// Evaluates the subnet change request to simulate the requested topology
    /// change. Command returns all the information about the subnet before
    /// and after the change.
    pub fn evaluate(self) -> Result<SubnetChange, NetworkError> {
        // If self.improve_count is set, we first try to optimize the subnet
        let (request_change, result_optimize) = if self.improve_count > 0 {
            let request_extend = Self {
                removed_nodes: Default::default(),
                ..self.clone()
            }
            .optimize(self.improve_count)?;

            let request_change = self
                .remove(
                    request_extend
                        .removed()
                        .iter()
                        .map(|n| n.id)
                        .collect::<Vec<_>>()
                        .as_slice(),
                )?
                .include_nodes(request_extend.added().iter().map(|n| n.id).collect());
            (request_change, Some(request_extend))
        } else {
            (self, None)
        };

        // Then we extend the subnet for the remaining nodes
        match (request_change, result_optimize) {
            (request_change, Some(result_optimize)) => {
                let result_extend =
                    request_change.resize(request_change.removed_nodes.len() - result_optimize.added().len(), 0)?;
                Ok(SubnetChange {
                    comment: if result_optimize.comment == result_extend.comment {
                        result_extend.comment
                    } else {
                        Some(format!(
                            "{}\n{}",
                            result_optimize.comment.unwrap_or_default(),
                            result_extend.comment.unwrap_or_default()
                        ))
                    },
                    run_log: result_optimize
                        .run_log
                        .into_iter()
                        .chain(result_extend.run_log)
                        .collect(),
                    ..result_extend
                })
            }
            (request_change, _) => request_change.resize(
                request_change.removed_nodes.len() - request_change.include_nodes.len(),
                0,
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SubnetChange {
    pub id: PrincipalId,
    pub old_nodes: Vec<Node>,
    pub new_nodes: Vec<Node>,
    pub min_nakamoto_coefficients: Option<MinNakamotoCoefficients>,
    pub comment: Option<String>,
    pub run_log: Vec<String>,
}

impl SubnetChange {
    pub fn added(&self) -> Vec<Node> {
        self.new_nodes
            .clone()
            .into_iter()
            .filter(|n| !self.old_nodes.contains(n))
            .collect()
    }

    pub fn removed(&self) -> Vec<Node> {
        self.old_nodes
            .clone()
            .into_iter()
            .filter(|n| !self.new_nodes.contains(n))
            .collect()
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
            removed_nodes: self
                .old_nodes
                .clone()
                .into_iter()
                .filter(|n| !self.new_nodes.contains(n))
                .collect(),
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
