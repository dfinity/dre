use crate::nakamoto::{self, Extendable, Feature};
use actix_web::http::StatusCode;
use actix_web::{FromRequest, HttpRequest, HttpResponse, ResponseError};
use async_trait::async_trait;
use futures_util::future::{err, ok, Ready};
use ic_base_types::PrincipalId;
use itertools::Itertools;
use lazy_static::lazy_static;
use reqwest::get;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fmt::{Debug, Display, Formatter};
use std::marker::Sync;
use std::str::FromStr;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

lazy_static! {
    static ref REGION_DATA: HashMap<String, DataCenterInfo> =
        serde_json::from_str(include_str!("static_region_data.json")).expect("Bad regions json");
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct ICApiSubnetNodesResponse {
    pub nodes: Vec<PubApiNode>,
}

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
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Subnet {
    pub id: PrincipalId,
    pub nodes: Vec<Node>,
}

impl Subnet {
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
            },
            removed,
        ))
    }

    fn add_nodes(&self, nodes: Vec<Node>) -> Self {
        Self {
            id: self.id,
            nodes: self.nodes.clone().into_iter().chain(nodes).collect(),
        }
    }
}

impl From<Subnet> for nakamoto::NakamotoScore {
    fn from(subnet: Subnet) -> Self {
        Self::from_vec_nodes(subnet.nodes)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NetworkError {
    NodeNotFound(PrincipalId),
    SubnetNotFound(PrincipalId),
    InsufficentAvailableNodes,
    DataRequestError,
    IllegalRequest(String),
}

impl Display for NetworkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self)
    }
}

impl ResponseError for NetworkError {
    fn error_response(&self) -> HttpResponse {
        match self {
            NetworkError::IllegalRequest(_input) => HttpResponse::build(StatusCode::BAD_REQUEST).json(self),
            NetworkError::InsufficentAvailableNodes => HttpResponse::InternalServerError().json(self),
            NetworkError::DataRequestError => HttpResponse::build(StatusCode::FAILED_DEPENDENCY).json(self),
            NetworkError::SubnetNotFound(_) | NetworkError::NodeNotFound(_) => HttpResponse::NotFound().json(self),
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::NodeNotFound(_) | Self::SubnetNotFound(_) => StatusCode::NOT_FOUND,
            Self::IllegalRequest(_) => StatusCode::BAD_REQUEST,
            Self::InsufficentAvailableNodes => StatusCode::INTERNAL_SERVER_ERROR,
            Self::DataRequestError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<reqwest::Error> for NetworkError {
    fn from(_: reqwest::Error) -> NetworkError {
        NetworkError::DataRequestError
    }
}

impl From<serde_json::Error> for NetworkError {
    fn from(_: serde_json::Error) -> NetworkError {
        NetworkError::DataRequestError
    }
}

#[async_trait]
pub trait AvailableNodesQuerier {
    async fn available_nodes(&self) -> Result<Vec<Node>, NetworkError>;
}

#[async_trait]
pub trait SubnetQuerier {
    async fn subnet(&self, id: &PrincipalId) -> Result<Subnet, NetworkError>;
    async fn subnet_of_nodes(&self, nodes: &[PrincipalId]) -> Result<Subnet, NetworkError>;
}

pub struct DashboardAgent {
    url: String,
    subnets: RwLock<HashMap<PrincipalId, (Vec<Node>, Instant)>>,
}

impl DashboardAgent {
    pub fn new(url: String) -> Self {
        Self {
            url,
            subnets: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl AvailableNodesQuerier for DashboardAgent {
    async fn available_nodes(&self) -> Result<Vec<Node>, NetworkError> {
        // TODO: REL-xxxx
        todo!()
    }
}

#[async_trait]
pub trait DecentralizationQuerier {
    async fn decentralization(&self, id: PrincipalId) -> Result<nakamoto::NakamotoScore, NetworkError>;
}

#[async_trait]
impl<T> DecentralizationQuerier for T
where
    T: SubnetQuerier + Sync,
{
    async fn decentralization(&self, id: PrincipalId) -> Result<nakamoto::NakamotoScore, NetworkError> {
        let subnet = self.subnet(&id).await?;
        let out = nakamoto::NakamotoScore::from_vec_nodes(subnet.nodes);
        Ok(out)
    }
}

#[async_trait]
impl SubnetQuerier for DashboardAgent {
    async fn subnet(&self, id: &PrincipalId) -> Result<Subnet, NetworkError> {
        // TODO: REL-xxxx
        let principalstr = id.to_string();
        let mut writer = self.subnets.write().await;
        let curr_time = Instant::now();
        if let Some((nodes, time)) = writer.get(id) {
            let elapsed = curr_time
                .checked_duration_since(*time)
                .expect("Failure in elapsed time measure");
            if elapsed <= Duration::from_secs(86400) {
                return Ok(Subnet {
                    id: *id,
                    nodes: nodes.clone(),
                });
            }
        };
        let ic_api_query_string = format!("{}?subnet={}", self.url.clone(), principalstr);
        let ic_api_dashboard_response = get(ic_api_query_string).await?;
        let ic_api_dashboard_nodes: serde_json::Value = ic_api_dashboard_response.json().await?;
        let ic_api_dashboard_nodes_parsed: ICApiSubnetNodesResponse = serde_json::from_value(ic_api_dashboard_nodes)?;
        writer.insert(
            *id,
            (
                ic_api_dashboard_nodes_parsed
                    .nodes
                    .iter()
                    .map(|x| x.clone().into())
                    .collect::<Vec<Node>>(),
                Instant::now(),
            ),
        );
        return Ok(Subnet {
            id: *id,
            nodes: writer.get(id).unwrap().clone().0,
        });
    }

    async fn subnet_of_nodes(&self, _: &[PrincipalId]) -> Result<Subnet, NetworkError> {
        unimplemented!()
    }
}

impl From<&mercury_management_types::Node> for Node {
    fn from(n: &mercury_management_types::Node) -> Self {
        Self {
            id: n.principal,
            features: nakamoto::NodeFeatures::from_iter(
                [
                    (
                        Feature::City,
                        n.operator
                            .datacenter
                            .as_ref()
                            .map(|d| d.city.clone())
                            .unwrap_or_else(|| "unknown".to_string()),
                    ),
                    (
                        Feature::Country,
                        n.operator
                            .datacenter
                            .as_ref()
                            .map(|d| d.country.clone())
                            .unwrap_or_else(|| "unknown".to_string()),
                    ),
                    (
                        Feature::Continent,
                        n.operator
                            .datacenter
                            .as_ref()
                            .map(|d| d.continent.clone())
                            .unwrap_or_else(|| "unknown".to_string()),
                    ),
                    (
                        Feature::DatacenterOwner,
                        n.operator
                            .datacenter
                            .as_ref()
                            .map(|d| d.owner.name.clone())
                            .unwrap_or_else(|| "unknown".to_string()),
                    ),
                    (
                        Feature::Datacenter,
                        n.operator
                            .datacenter
                            .as_ref()
                            .map(|d| d.name.clone())
                            .unwrap_or_else(|| "unknown".to_string()),
                    ),
                    (Feature::NodeProvider, n.operator.provider.principal.to_string()),
                ]
                .into_iter(),
            ),
            dfinity_owned: n.dfinity_owned,
        }
    }
}

pub struct InternalDashboardAgent {
    url: String,
}

impl InternalDashboardAgent {
    pub fn new(url: String) -> Self {
        Self { url }
    }
}
impl InternalDashboardAgent {
    async fn subnets(&self) -> Result<HashMap<PrincipalId, mercury_management_types::Subnet>, NetworkError> {
        reqwest::get(format!("{}/subnets", self.url.clone()))
            .await?
            .json::<HashMap<PrincipalId, mercury_management_types::Subnet>>()
            .await
            .map_err(NetworkError::from)
    }
}

#[async_trait]
impl SubnetQuerier for InternalDashboardAgent {
    async fn subnet(&self, id: &PrincipalId) -> Result<Subnet, NetworkError> {
        self.subnets()
            .await?
            .get(id)
            .map(|s| Subnet {
                id: s.principal,
                nodes: s.nodes.iter().map(Node::from).collect(),
            })
            .ok_or(NetworkError::SubnetNotFound(*id))
    }

    async fn subnet_of_nodes(&self, nodes: &[PrincipalId]) -> Result<Subnet, NetworkError> {
        self.subnets()
            .await?
            .values()
            .find_map(|s| {
                if nodes.iter().all(|n| s.nodes.iter().any(|sn| sn.principal == *n)) {
                    Some(Subnet {
                        id: s.principal,
                        nodes: s.nodes.iter().map(Node::from).collect(),
                    })
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                NetworkError::IllegalRequest("No single subnet has all the nodes requested to be replaced".to_string())
            })
    }
}

#[async_trait]
impl AvailableNodesQuerier for InternalDashboardAgent {
    async fn available_nodes(&self) -> Result<Vec<Node>, NetworkError> {
        Ok(reqwest::get(format!("{}/nodes", self.url.clone()))
            .await?
            .json::<HashMap<PrincipalId, mercury_management_types::Node>>()
            .await?
            .values()
            .sorted_by(|a, b| a.principal.cmp(&b.principal))
            .filter(|n| n.subnet == None && n.proposal.is_none())
            .map(Node::from)
            .collect::<Vec<_>>())
    }
}

const DEFAULT_SUBNET_SIZE: usize = 13;
const MINIMUM_ALLOWED_NAKAMOTO_SCORE: f64 = 0.0;

pub struct SubnetsManager<T: AvailableNodesQuerier + SubnetQuerier> {
    pub network: T,
}

impl<T: AvailableNodesQuerier + SubnetQuerier> SubnetsManager<T> {
    pub fn new(network: T) -> Self {
        Self { network }
    }

    pub async fn subnet(&self, subnet_id: PrincipalId) -> Result<SubnetChangeRequest, NetworkError> {
        Ok(SubnetChangeRequest {
            available_nodes: self.network.available_nodes().await?,
            subnet: self.network.subnet(&subnet_id).await?,
            ..Default::default()
        })
    }

    pub async fn create(&self, size: usize) -> Result<Vec<Node>, NetworkError> {
        SubnetChangeRequest {
            available_nodes: self.network.available_nodes().await?,
            ..Default::default()
        }
        .extend(size)
        .map(|s| s.new_nodes)
    }
}

impl FromRequest for InternalDashboardAgent {
    type Error = DecentralizationError;
    type Future = Ready<Result<InternalDashboardAgent, Self::Error>>;
    fn from_request(
        _: &HttpRequest,
        _: &mut actix_web::dev::Payload,
    ) -> Ready<Result<InternalDashboardAgent, DecentralizationError>> {
        let data = env::var("REGISTRY_API_URL");
        match data {
            Ok(v) => ok(InternalDashboardAgent::new(v)),
            Err(_) => err(DecentralizationError::FeatureNotAvailable),
        }
    }
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
            serde_json::from_str("{\"message\": \"Feature not available. For access contact the administrator\"}")
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

    async fn create_subnet(&self) -> Result<Vec<Node>, NetworkError> {
        SubnetChangeRequest {
            available_nodes: self.available_nodes().await?,
            ..Default::default()
        }
        .extend_default()
        .map(|s| s.new_nodes)
    }
}

#[derive(Default, Clone)]
pub struct SubnetChangeRequest {
    subnet: Subnet,
    available_nodes: Vec<Node>,
    include_nodes: Vec<PrincipalId>,
    removed_nodes: Vec<Node>,
    improve_count: usize,
}

impl SubnetChangeRequest {
    pub fn subnet(&self) -> Subnet {
        self.subnet.clone()
    }

    pub fn include_nodes(self, nodes: Vec<PrincipalId>) -> Self {
        Self {
            include_nodes: self.include_nodes.into_iter().chain(nodes).collect(),
            ..self
        }
    }

    pub fn exclude_nodes(self, nodes: Vec<PrincipalId>) -> Self {
        Self {
            available_nodes: self
                .available_nodes
                .into_iter()
                .filter(|n| !nodes.contains(&n.id))
                .collect(),
            ..self
        }
    }

    fn extend(&self, extension_size: usize) -> Result<SubnetChange, NetworkError> {
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

        let extension = self
            .subnet
            .nodes
            .clone()
            .into_iter()
            .chain(included_nodes.clone())
            .collect::<Vec<Node>>()
            .best_extension(extension_size, &available_nodes)
            .ok_or(NetworkError::InsufficentAvailableNodes)?;

        if nakamoto::NakamotoScore::from_vec_nodes(extension.clone()).total() < MINIMUM_ALLOWED_NAKAMOTO_SCORE {
            return Err(NetworkError::InsufficentAvailableNodes);
        }

        Ok(SubnetChange {
            id: self.subnet.id,
            old_nodes: self
                .subnet
                .nodes
                .clone()
                .into_iter()
                .chain(self.removed_nodes.clone())
                .collect(),
            new_nodes: self.subnet.add_nodes(extension).add_nodes(included_nodes).nodes,
        })
    }

    pub fn replace(self, nodes: &[PrincipalId]) -> Result<SubnetChange, NetworkError> {
        let (subnet, mut removed_nodes) = self.subnet.remove_nodes(nodes)?;
        Self { subnet, ..self }.extend(removed_nodes.len()).map(|mut sc| {
            sc.old_nodes.append(&mut removed_nodes);
            sc
        })
    }

    pub fn extend_default(&self) -> Result<SubnetChange, NetworkError> {
        self.extend(DEFAULT_SUBNET_SIZE.saturating_sub(self.subnet.nodes.len()))
    }

    pub fn optimize(self, max_replacements: usize) -> Result<SubnetChange, NetworkError> {
        self.subnet
            .nodes
            .iter()
            .combinations(max_replacements)
            .map(|nodes| {
                let mut change = self.clone();
                change
                    .available_nodes
                    .append(&mut nodes.iter().map(|n| (*n).clone()).collect::<Vec<_>>());
                change.replace(nodes.iter().map(|n| n.id).collect::<Vec<_>>().as_slice())
            })
            .filter_map(|r| r.ok())
            .max_by(|sc1, sc2| {
                nakamoto::NakamotoScore::from_vec_nodes(sc1.new_nodes.clone())
                    .total()
                    .partial_cmp(&nakamoto::NakamotoScore::from_vec_nodes(sc2.new_nodes.clone()).total())
                    .unwrap()
            })
            .ok_or(NetworkError::InsufficentAvailableNodes)
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

pub struct SubnetChange {
    pub id: PrincipalId,
    pub old_nodes: Vec<Node>,
    pub new_nodes: Vec<Node>,
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

    pub fn before(&self) -> Subnet {
        Subnet {
            id: self.id,
            nodes: self.old_nodes.clone(),
        }
    }

    pub fn after(&self) -> Subnet {
        Subnet {
            id: self.id,
            nodes: self.new_nodes.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PubApiNode {
    dc: String,
    location_name: Option<String>,
    node_id: String,
    node_operator_id: Option<String>,
    node_operator_name: Option<String>,
    node_provider_id: Option<String>,
    status: Value,
}

impl From<PubApiNode> for Node {
    fn from(src: PubApiNode) -> Node {
        let dc = src.dc.clone();
        let dcinfo: DataCenterInfo = REGION_DATA.get(&dc).unwrap_or(&DataCenterInfo::default()).clone();
        let (city, country, continent) = (dcinfo.city, dcinfo.country, dcinfo.continent);

        let feats = [
            (Feature::City, city.into()),
            (Feature::Country, country.into()),
            (Feature::Continent, continent.into()),
            (Feature::Datacenter, dc.into()),
            (Feature::DatacenterOwner, src.node_operator_name),
            (Feature::NodeProvider, src.node_provider_id),
        ]
        .iter()
        .cloned()
        .filter_map(|(f, v)| v.map(|v| (f, v)))
        .collect();

        Node {
            id: PrincipalId::from_str(&src.node_id).unwrap_or_default(),
            features: feats,
            // HACK: not possible to determine from the public dashboard API. The value is not relevant for this
            // implementation.
            dfinity_owned: false,
        }
    }
}
