use std::{path::PathBuf, str::FromStr};

use csv::Reader;
use decentralization::SubnetChangeResponse;
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

    pub fn summary_for_change(&self, change: &SubnetChangeResponse) -> anyhow::Result<String> {
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
                let key = key.trim_start_matches("limit_");

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
            writeln!(output, "- `{}` [health: {}]", node_id, health)?;
        }
        writeln!(output, "\nNodes added:")?;
        for node_id in &change.node_ids_added {
            let health = change
                .health_of_nodes
                .get(node_id)
                .map(|h| h.to_string().to_lowercase())
                .unwrap_or("unknown".to_string());
            writeln!(output, "- `{}` [health: {}]", node_id, health)?;
        }

        writeln!(output, "## Attribute wise view of the changes")?;
        writeln!(
            output,
            "| {} |\n| {} |",
            change.feature_diff.keys().map(|key| format!("{key} changes")).join(" | "),
            change
                .feature_diff
                .keys()
                .map(|key| format!("{key} changes"))
                .map(|extended| "-".repeat(extended.len()))
                .join(" | "),
        )?;

        let max_len = change.feature_diff.values().map(|inner| inner.len()).max().unwrap_or(0);
        let num_features = change.feature_diff.len();

        let padded_features: Vec<(NodeFeature, Vec<(String, (usize, usize))>)> = change
            .feature_diff
            .clone()
            .into_iter()
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
                let (value, (before, after)) = &padded_features[column].1[row];
                if !value.is_empty() {
                    if before == after {
                        write!(line, "{value} {before}")?;
                    } else {
                        write!(line, "{value} {before} -> {after}")?;
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
