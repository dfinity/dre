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
        }
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct DecentralizedSubnet {
    pub id: PrincipalId,
    pub nodes: Vec<Node>,
    pub min_nakamoto_coefficients: Option<MinNakamotoCoefficients>,
    pub comment: Option<String>,
}

impl DecentralizedSubnet {
    fn remove_nodes(&self, nodes: &[PrincipalId]) -> Result<(Self, Vec<Node>), NetworkError> {
        let mut new_subnet_nodes = self.nodes.clone();
        let mut removed = Vec::new();
        for node in nodes {
            if let Some(index) = new_subnet_nodes.iter().position(|n| n.id == *node) {
                removed.push(new_subnet_nodes.remove(index));
            } else {
                return Err(NetworkError::NodeNotFound(*node));
            }
        }
        Ok((
            Self {
                id: self.id,
                nodes: new_subnet_nodes,
                min_nakamoto_coefficients: self.min_nakamoto_coefficients.clone(),
                comment: self.comment.clone(),
            },
            removed,
        ))
    }

    fn add_nodes(&self, nodes: Vec<Node>) -> Self {
        Self {
            id: self.id,
            nodes: self.nodes.clone().into_iter().chain(nodes).collect(),
            min_nakamoto_coefficients: self.min_nakamoto_coefficients.clone(),
            comment: self.comment.clone(),
        }
    }

    fn with_min_nakamoto_coefficients(self, min_nakamoto_coefficients: &Option<MinNakamotoCoefficients>) -> Self {
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

        let dfinity_owned_nodes_count: usize = nodes.iter().map(|n| n.dfinity_owned as usize).sum();

        let nakamoto_scores = Self::_calc_nakamoto_score(nodes);
        let subnet_id_str = subnet_id.to_string();
        if subnet_id_str == *"tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe"
            && dfinity_owned_nodes_count < 3
        {
            checks.push(format!(
                "Mainnet NNS subnet should have at least 3 DFINITY-owned nodes, got {}",
                dfinity_owned_nodes_count
            ));
            penalties += (3 - dfinity_owned_nodes_count) * 1000;
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
        if dfinity_owned_nodes_count < 1 {
            checks.push("DFINITY-owned node missing".to_string());
            penalties += 1000;
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
        let mut run_log = Vec::new();

        let mut nodes_initial = self.nodes.clone();
        let mut nodes_available = available_nodes.to_vec();
        let orig_available_nodes_len = nodes_available.len();
        let mut nodes_after_extension = self.nodes.clone();
        let mut comment = None;

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

        for _ in 0..num_nodes_to_add {
            let mut sorted_good_nodes: Vec<SortResult> = nodes_available
                .iter()
                .enumerate()
                .filter_map(|(index, node)| {
                    let candidate_subnet_nodes: Vec<Node> = nodes_initial.iter().chain([node]).cloned().collect();
                    match Self::_check_business_rules_for_nodes(
                        &self.id,
                        &candidate_subnet_nodes,
                        &self.min_nakamoto_coefficients,
                    ) {
                        Ok((business_rules_penalty, business_rules_log)) => {
                            let new_score = Self::_calc_nakamoto_score(&candidate_subnet_nodes);
                            let mut penalty = business_rules_penalty;
                            if node.dfinity_owned {
                                penalty += 100
                            };
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

            println!("Sorted candidate nodes, with the best candidate at the end:");
            println!("     <node-id>                                                      <penalty>  <Nakamoto score>");
            for s in &sorted_good_nodes {
                println!(" -=> {} {} {}", s.node.id, s.penalty, s.score);
            }
            // TODO: if more than one candidate returns the same nakamoto score, pick the
            // one that improves the feature diversity
            let best_result = sorted_good_nodes.pop();
            match best_result {
                Some(best_result) => {
                    nodes_available.swap_remove(best_result.index);
                    nodes_after_extension.push(best_result.node.clone());
                    nodes_initial.push(best_result.node.clone());
                    if best_result.penalty != 0 {
                        comment = Some(format!(
                            "Best result has penalty {}. Details of the business rules checks:\n{}",
                            best_result.penalty,
                            best_result.business_rules_log.join("\n")
                        ));
                    } else {
                        comment = None;
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
        assert_eq!(orig_available_nodes_len - nodes_available.len(), num_nodes_to_add);

        Ok(Self {
            id: self.id,
            nodes: nodes_after_extension,
            min_nakamoto_coefficients: self.min_nakamoto_coefficients.clone(),
            comment,
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
            min_nakamoto_coefficients: None,
            comment: None,
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

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum DecentralizationError {
    FeatureNotAvailable,
}

impl Display for DecentralizationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self)
    }
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
        .extend(size)
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
                .filter(|n| {
                    let mut should_include_node = true;
                    for exclude_string in &exclude_nodes_or_features {
                        // Exclude the node if
                        if n.id.to_string() == *exclude_string {
                            // The node id matches an entry from the exclude list
                            should_include_node = false;
                            info!("Excluding node {} due to an excluded node id", n.id);
                        } else {
                            // Or if any of the node features matches *exactly* an entry from the exclude
                            // list
                            for (_, feat_val) in n.get_features().feature_map {
                                if feat_val == *exclude_string {
                                    should_include_node = false;
                                    info!("Excluding node {} due to excluded feature {}", n.id, feat_val);
                                }
                            }
                        }
                    }
                    should_include_node
                })
                .collect(),
            ..self
        }
    }

    pub fn with_min_nakamoto_coefficients(self, min_nakamoto_coefficients: Option<MinNakamotoCoefficients>) -> Self {
        Self {
            min_nakamoto_coefficients,
            ..self
        }
    }

    pub fn extend(&self, extension_size: usize) -> Result<SubnetChange, NetworkError> {
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

        let extended_subnet = self
            .subnet
            .add_nodes(included_nodes)
            .with_min_nakamoto_coefficients(&self.min_nakamoto_coefficients)
            .new_extended_subnet(extension_size, &available_nodes)
            .map_err(|e| NetworkError::ExtensionFailed(e.to_string()))?;

        let subnet_change = SubnetChange {
            id: self.subnet.id,
            old_nodes: self
                .subnet
                .nodes
                .clone()
                .into_iter()
                .chain(self.removed_nodes.clone())
                .collect(),
            new_nodes: extended_subnet.nodes,
            min_nakamoto_coefficients: self.min_nakamoto_coefficients.clone(),
            comment: extended_subnet.comment,
        };
        info!("Subnet {} extend {}", self.subnet.id, subnet_change);
        Ok(subnet_change)
    }

    pub fn replace(self, nodes: &[PrincipalId]) -> Result<SubnetChange, NetworkError> {
        let (subnet, mut removed_nodes) = self.subnet.remove_nodes(nodes)?;

        Self { subnet, ..self }.extend(removed_nodes.len()).map(|mut sc| {
            sc.old_nodes.append(&mut removed_nodes);
            sc
        })
    }

    pub fn optimize(self, max_replacements: usize) -> Result<SubnetChange, NetworkError> {
        let max_replacements = if max_replacements > 3 {
            warn!("Limiting the max replacements to 3 to prevent DOS");
            3
        } else {
            max_replacements
        };

        let results = self.subnet.nodes.iter().combinations(max_replacements).map(|nodes| {
            let mut change = self.clone();
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
            None => Err(NetworkError::ExtensionFailed(format!(
                "Optimize failed, could not find any suitable solution for the request\n{}",
                errs.join("\n")
            ))),
        }
    }

    pub fn remove(self, nodes: &[PrincipalId]) -> Result<SubnetChangeRequest, NetworkError> {
        let (subnet, removed_nodes) = self.subnet.remove_nodes(nodes)?;
        Ok(SubnetChangeRequest {
            subnet,
            removed_nodes: self.removed_nodes.into_iter().chain(removed_nodes).collect(),
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
        let change = if self.improve_count > 0 {
            let change = Self {
                removed_nodes: Default::default(),
                ..self.clone()
            }
            .optimize(self.improve_count)?;
            self.remove(change.removed().iter().map(|n| n.id).collect::<Vec<_>>().as_slice())?
                .include_nodes(change.added().iter().map(|n| n.id).collect())
        } else {
            self
        };

        change.extend(change.removed_nodes.len() - change.include_nodes.len())
    }
}

#[derive(Debug, Clone)]
pub struct SubnetChange {
    pub id: PrincipalId,
    pub old_nodes: Vec<Node>,
    pub new_nodes: Vec<Node>,
    pub min_nakamoto_coefficients: Option<MinNakamotoCoefficients>,
    pub comment: Option<String>,
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
            min_nakamoto_coefficients: self.min_nakamoto_coefficients.clone(),
            comment: self.comment.clone(),
        }
    }

    pub fn after(&self) -> DecentralizedSubnet {
        DecentralizedSubnet {
            id: self.id,
            nodes: self.new_nodes.clone(),
            min_nakamoto_coefficients: self.min_nakamoto_coefficients.clone(),
            comment: self.comment.clone(),
        }
    }
}

impl Display for SubnetChange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", SubnetChangeResponse::from(self))
    }
}
