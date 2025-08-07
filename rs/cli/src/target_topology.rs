use std::{path::PathBuf, str::FromStr};

use decentralization::SubnetChangeResponse;
use ic_types::PrincipalId;
use serde::Deserialize;

use crate::store::Store;

#[derive(Debug, Clone, Default)]
pub enum TargetTopologyOption {
    #[default]
    Current,
    ProposalId(u64),
    Path(PathBuf),
}

impl FromStr for TargetTopologyOption {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Try parsing as a number
        if let Ok(id) = s.parse::<u64>() {
            return Ok(TargetTopologyOption::ProposalId(id));
        }

        // Otherwise treat as path
        Ok(TargetTopologyOption::Path(PathBuf::from(s)))
    }
}

#[derive(Debug, Clone, Deserialize)]
struct TargetTopologyEntry {
    subnet_type: String,
    subnet_id: PrincipalId,
    subnet_size: u8,
    is_sev: bool,
    #[serde(rename = "subnet_limit_node_provider")]
    node_provider_limit: u8,
    #[serde(rename = "subnet_limit_data_center")]
    data_center_limit: u8,
    #[serde(rename = "subnet_limit_data_center_provider")]
    data_center_provider_limit: u8,
    #[serde(rename = "subnet_limit_city")]
    country_limit: u8,
}

pub struct TargetTopology {
    entries: Vec<TargetTopologyEntry>,
}

impl TargetTopology {
    fn new() -> Self {
        Self { entries: vec![] }
    }

    pub async fn from_option(option: TargetTopologyOption, store: Store) -> anyhow::Result<Self> {
        match option {
            TargetTopologyOption::Current => Self::from_current(store),
            TargetTopologyOption::ProposalId(pid) => Self::from_proposal(pid, store),
            TargetTopologyOption::Path(path_buf) => Self::from_path(path_buf),
        }
    }

    fn from_path(path: PathBuf) -> anyhow::Result<Self> {
        Ok(Self::new())
    }

    fn from_current(store: Store) -> anyhow::Result<Self> {
        Ok(Self::new())
    }

    fn from_proposal(proposal: u64, store: Store) -> anyhow::Result<Self> {
        Ok(Self::new())
    }

    pub fn for_change(&self, change: &SubnetChangeResponse) -> String {
        "TBD".to_string()
    }
}
