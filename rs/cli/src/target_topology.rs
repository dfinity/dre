use std::fmt::Write;
use std::{path::PathBuf, str::FromStr};

use csv::Reader;
use decentralization::SubnetChangeResponse;
use ic_types::PrincipalId;
use serde::{Deserialize, Serialize};

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

        let mut output = String::new();

        writeln!(output, "## Target topology entry\n")?;
        writeln!(output, "Target topology used was established in proposal [{}](https://dashboard.internetcomputer.org/proposal/{})\nSubnet id: [`{subnet_id}`](https://dashboard.internetcomputer.org/network/subnets/{subnet_id})\n", self.proposal, self.proposal)?;

        self.write_target_topology_table(&mut output, entry)?;

        change.write_details(&mut output)?;

        change.write_attribute_table(&mut output)?;

        writeln!(output, "> **Note:** Each column represents changes for a single attribute type and is independent from the others. Rows are used only for layout purposes there is no correlation between entries in the same row.")?;

        Ok(output.trim().to_string())
    }

    fn write_target_topology_table<W: Write>(&self, f: &mut W, entry: &TargetTopologyEntry) -> std::fmt::Result {
        let mut serialized_entry = serde_json::to_value(entry).unwrap();
        let serialized_entry = serialized_entry.as_object_mut().unwrap();

        serialized_entry.remove("subnet_id");

        let columns: Vec<_> = serialized_entry
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

        let mut table = tabular::Table::new(&columns.iter().map(|_| "    {:<}").collect::<Vec<_>>().join(""));
        table.add_row(columns.iter().fold(tabular::Row::new(), |acc, k| acc.with_cell(k.to_string())));
        table.add_row(
            columns
                .iter()
                .fold(tabular::Row::new(), |acc, k| acc.with_cell("-".repeat(k.to_string().len()))),
        );

        table.add_row(serialized_entry.values().fold(tabular::Row::new(), |acc, k| {
            acc.with_cell(match k {
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.to_string(),
                other => panic!("Not known how to format {other:?}"),
            })
        }));

        writeln!(f, "\n\n```\n{}```", table)?;

        Ok(())
    }
}
