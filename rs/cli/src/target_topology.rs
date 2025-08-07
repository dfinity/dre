use std::{path::PathBuf, str::FromStr, sync::Arc};

use csv::Reader;
use decentralization::SubnetChangeResponse;
use ic_management_backend::lazy_registry::LazyRegistry;
use ic_management_types::NodeFeature;
use ic_types::PrincipalId;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::io::Write;

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

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TargetTopologyEntry {
    subnet_type: String,
    subnet_id: PrincipalId,
    subnet_size: u8,
    #[serde(rename = "subnet_limit_node_provider")]
    node_provider_limit: u8,
    #[serde(rename = "subnet_limit_data_center")]
    data_center_limit: u8,
    #[serde(rename = "subnet_limit_data_center_provider")]
    data_center_provider_limit: u8,
    #[serde(rename = "subnet_limit_country")]
    country_limit: u8,
}

pub struct TargetTopology {
    entries: Vec<TargetTopologyEntry>,
    proposal: u64,
}

impl TargetTopology {
    fn new() -> Self {
        Self {
            entries: vec![],
            proposal: 0,
        }
    }

    pub async fn from_option(option: TargetTopologyOption, store: Store) -> anyhow::Result<Self> {
        match option {
            TargetTopologyOption::Current => Self::from_current(store),
            TargetTopologyOption::ProposalId(pid) => Self::from_proposal(pid, store),
            TargetTopologyOption::Path(path_buf) => Self::from_path(path_buf),
        }
    }

    fn from_path(path: PathBuf) -> anyhow::Result<Self> {
        let contents = fs_err::read_to_string(&path).map_err(anyhow::Error::from)?;
        let mut reader = csv::Reader::from_reader(contents.as_bytes());

        let stem = path
            .file_name()
            .ok_or(anyhow::anyhow!("Topology file is not an OsStr"))?
            .to_str()
            .ok_or(anyhow::anyhow!("OsStr is not a valid utf8 string"))?
            .to_string();
        let stem = stem.trim_end_matches(".csv");
        Self::from_reader(
            &mut reader,
            stem.parse()
                .map_err(|e| anyhow::anyhow!("Unexpected topology file name. It should be a u64 followed by a .csv. {e:?}"))?,
        )
    }

    fn from_current(_store: Store) -> anyhow::Result<Self> {
        Ok(Self::new())
    }

    fn from_proposal(_proposal: u64, _store: Store) -> anyhow::Result<Self> {
        Ok(Self::new())
    }

    fn from_reader(reader: &mut Reader<&[u8]>, proposal: u64) -> anyhow::Result<Self> {
        let mut topology = Self::new();
        topology.proposal = proposal;

        for record in reader.deserialize() {
            let record: TargetTopologyEntry = record?;
            topology.entries.push(record);
        }

        Ok(topology)
    }

    pub async fn summary_for_change(&self, change: &SubnetChangeResponse, registry: Arc<dyn LazyRegistry>) -> anyhow::Result<String> {
        let subnet_id = change.subnet_id.ok_or(anyhow::anyhow!("Subnet id is missing"))?;
        let entry = self
            .entries
            .iter()
            .find(|t| PartialEq::eq(&t.subnet_id, &subnet_id))
            .ok_or(anyhow::anyhow!("Topology entry for subnet {subnet_id} is missing."))?;

        let mut output: Vec<u8> = vec![];

        writeln!(output, "## Target topology entry\nTarget topology used was established in proposal [{}](https://dashboard.internetcomputer.org/proposal/{})\nSubnet id: [`{subnet_id}`](https://dashboard.internetcomputer.org/network/subnets/{subnet_id})", self.proposal, self.proposal)?;
        let mut serialized_entry = serde_json::to_value(&entry)?;
        let serialized_entry = serialized_entry.as_object_mut().ok_or(anyhow::anyhow!("Unexpected row serialization."))?;

        serialized_entry.remove("subnet_id");

        let keys: Vec<_> = serialized_entry
            .keys()
            .map(|key| {
                let key = key.trim_start_matches("subnet_");
                let key = if key.starts_with("liimt_") {
                    let key = key.trim_start_matches("limit_");
                    format!("{key}_limit")
                } else {
                    key.to_string()
                };

                key.replace("_", " ")
            })
            .collect();

        writeln!(
            output,
            "| {} |\n| {} |\n| {} |",
            keys.join(" | "),
            keys.iter().map(|header| "-".repeat(header.len())).join(" | "),
            serialized_entry
                .values()
                .map(|val| match val {
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::String(s) => s.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    other => panic!("Not known how to format {other:?}"),
                })
                .join(" | ")
        )?;

        writeln!(output, "## Node replacement details")?;
        writeln!(output, "Nodes removed:")?;
        for node_id in &change.node_ids_removed {
            let health = change
                .health_of_nodes
                .get(node_id)
                .map(|h| h.to_string().to_lowercase())
                .unwrap_or("unknown".to_string());
            let link = link_node_feature(&NodeFeature::NodeId, &node_id.to_string()).unwrap();
            writeln!(output, "- [`{node_id}`]({link}) [health: {health}]")?;
        }
        writeln!(output, "\nNodes added:")?;
        for node_id in &change.node_ids_added {
            let health = change
                .health_of_nodes
                .get(node_id)
                .map(|h| h.to_string().to_lowercase())
                .unwrap_or("unknown".to_string());
            let link = link_node_feature(&NodeFeature::NodeId, &node_id.to_string()).unwrap();
            writeln!(output, "- [`{node_id}`]({link}) [health: {health}]")?;
        }

        let mut feature_diffs = change.feature_diff.clone();
        feature_diffs.shift_remove(&NodeFeature::Area);

        let keys: Vec<_> = feature_diffs
            .keys()
            .map(|key| {
                let key = key.to_string();
                let key = key.replace("_", " ");
                format!("{key} changes")
            })
            .collect();
        writeln!(output, "## Attribute wise view of the changes")?;
        writeln!(
            output,
            "| {} |\n| {} |",
            keys.join(" | "),
            keys.iter().map(|extended| "-".repeat(extended.len())).join(" | "),
        )?;

        let max_len = feature_diffs.values().map(|inner| inner.len()).max().unwrap_or(0);
        let num_features = feature_diffs.len();

        let padded_features: Vec<(NodeFeature, Vec<(String, (usize, usize))>)> = feature_diffs
            .clone()
            .into_iter()
            .filter(|(feat, _)| feat != &NodeFeature::Area)
            .map(|(key, inner_map)| {
                let mut entries: Vec<_> = inner_map.into_iter().collect();
                while entries.len() < max_len {
                    entries.push(("".to_string(), (0, 0)));
                }
                (key, entries)
            })
            .collect();

        for row in 0..max_len {
            let mut line: Vec<u8> = vec![];
            write!(line, "| ")?;
            for column in 0..num_features {
                let (feature, column_data) = &padded_features[column];
                let (value, (before, after)) = &column_data[row];

                if !value.is_empty() {
                    let enriched_key = enrich_key(feature, value, registry.clone()).await?;
                    let displayed_value = link_node_feature(feature, value)
                        .map(|link| format!("[`{enriched_key}`]({link})"))
                        .unwrap_or(format!("`{enriched_key}`"));
                    if before == after {
                        write!(line, "{displayed_value}  {before}")?;
                    } else {
                        write!(line, "{displayed_value}  {before} -> {after}")?;
                    }
                }
                write!(line, " |")?;
            }
            write!(line, "\n")?;
            output.extend_from_slice(&line);
        }

        String::from_utf8(output).map_err(anyhow::Error::from)
    }
}

fn link_node_feature(feature: &NodeFeature, value: &str) -> Option<String> {
    match feature {
        NodeFeature::NodeId => Some(format!("https://dashboard.internetcomputer.org/network/nodes/{value}")),
        NodeFeature::NodeProvider => Some(format!("https://dashboard.internetcomputer.org/network/providers/{value}")),
        NodeFeature::DataCenter => Some(format!("https://dashboard.internetcomputer.org/network/centers/{value}")),
        _ => None,
    }
}

async fn enrich_key(feature: &NodeFeature, value: &str, registry: Arc<dyn LazyRegistry>) -> anyhow::Result<String> {
    match feature {
        NodeFeature::NodeProvider => {
            let nodes = registry.nodes().await?;
            let provider = nodes.values().find_map(|node| {
                if node.operator.provider.principal.to_string() == value {
                    node.operator.provider.name.clone()
                } else {
                    None
                }
            });

            let display_value = value.split_once("-").unwrap().0;
            Ok(provider.map(|name| format!("{display_value} - ({name})",)).unwrap_or(value.to_string()))
        }
        NodeFeature::DataCenter => {
            let dcs = registry.get_datacenters()?;

            let dc = dcs.iter().find_map(|dc| {
                if dc.id == value {
                    Some(dc.region.rsplit_once(",").unwrap().1)
                } else {
                    None
                }
            });

            Ok(dc.map(|town| format!("{value} - ({town})")).unwrap_or(value.to_string()))
        }
        _ => Ok(value.to_string()),
    }
}
