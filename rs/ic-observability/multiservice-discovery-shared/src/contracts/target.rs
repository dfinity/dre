use std::{
    collections::{BTreeMap, BTreeSet},
    hash::Hash,
    net::SocketAddr,
};

use ic_types::{NodeId, PrincipalId, SubnetId};
use serde::{Deserialize, Serialize};
use service_discovery::{job_types::JobType, TargetGroup};

use super::DataContract;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct TargetDto {
    pub node_id: NodeId,
    pub ic_name: String,
    /// A set of targets to be scraped that share the same labels.
    pub targets: BTreeSet<SocketAddr>,
    /// A set of labels that are associated with the targets listed in
    /// `socket_addr`.
    pub subnet_id: Option<SubnetId>,

    pub dc_id: String,
    pub operator_id: PrincipalId,
    pub node_provider_id: PrincipalId,
    pub jobs: Vec<JobType>,
    pub custom_labels: BTreeMap<String, String>,
    pub name: String,
    pub is_api_bn: bool,
    pub domain: Option<String>,
}

impl DataContract for TargetDto {
    fn get_name(&self) -> String {
        self.ic_name.to_string()
    }

    fn get_id(&self) -> String {
        self.node_id.to_string()
    }

    fn get_target_name(&self) -> String {
        self.name.to_string()
    }
}

pub fn map_to_target_dto(
    value: &TargetGroup,
    job_type: JobType,
    custom_labels: BTreeMap<String, String>,
    name: String,
    def_name: String,
) -> TargetDto {
    TargetDto {
        name,
        node_id: value.node_id,
        ic_name: def_name,
        targets: value.targets.clone(),
        subnet_id: value.subnet_id,
        dc_id: value.dc_id.clone(),
        operator_id: value.operator_id,
        node_provider_id: value.node_provider_id,
        jobs: vec![job_type],
        custom_labels,
        is_api_bn: value.is_api_bn,
        domain: value.domain.clone(),
    }
}

impl From<&TargetDto> for TargetGroup {
    fn from(value: &TargetDto) -> Self {
        Self {
            dc_id: value.dc_id.clone(),
            ic_name: value.ic_name.clone(),
            node_id: value.node_id,
            node_provider_id: value.node_provider_id,
            operator_id: value.operator_id,
            subnet_id: value.subnet_id,
            targets: value.targets.clone(),
            is_api_bn: value.is_api_bn,
            domain: value.domain.clone(),
        }
    }
}

impl From<&TargetGroup> for TargetDto {
    fn from(value: &TargetGroup) -> Self {
        Self {
            name: "".to_string(),
            custom_labels: BTreeMap::new(),
            jobs: vec![],
            node_id: value.node_id,
            ic_name: value.ic_name.clone(),
            targets: value.targets.clone(),
            subnet_id: value.subnet_id,
            dc_id: value.dc_id.clone(),
            operator_id: value.operator_id,
            node_provider_id: value.node_provider_id,
            is_api_bn: value.is_api_bn,
            domain: value.domain.clone(),
        }
    }
}

impl Hash for TargetDto {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.node_id.hash(state);
        self.ic_name.hash(state);
        self.targets.hash(state);
        self.subnet_id.hash(state);
        self.dc_id.hash(state);
        self.operator_id.hash(state);
        self.node_provider_id.hash(state);
        self.jobs.hash(state);
        self.name.hash(state);
        self.custom_labels.hash(state);
    }
}
