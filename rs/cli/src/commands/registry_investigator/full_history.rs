use crate::commands::registry_investigator::{AuthRequirement, DecodedRecord, KeyType, RegistryDiagnoser, serialize_decoded_record};
use crate::exe::ExecutableCommand;
use crate::exe::args::GlobalArgs;
use chrono::{DateTime, Utc};
use clap::{Args, ValueEnum};
use colored::Colorize;
use ic_canisters::registry::RegistryCanisterWrapper;
use ic_interfaces_registry::{RegistryClient, RegistryVersionedRecord};
use ic_protobuf::registry::{
    api_boundary_node::v1::ApiBoundaryNodeRecord,
    dc::v1::DataCenterRecord,
    hostos_version::v1::HostosVersionRecord,
    node::v1::NodeRecord,
    node_operator::v1::NodeOperatorRecord,
    node_rewards::v2::NodeRewardsTable,
    replica_version::v1::{BlessedReplicaVersions, ReplicaVersionRecord},
    subnet::v1::{SubnetListRecord, SubnetRecord},
};
use log::info;
use prost::Message;
use similar::TextDiff;
use strum::Display;

#[derive(Args, Debug)]
pub struct FullHistory {
    #[clap(long)]
    key_type: KeyType,

    #[clap(long)]
    key_value: Option<String>,

    #[clap(long, default_value_t = DisplayMode::Full, ignore_case = true)]
    display: DisplayMode,
}

impl ExecutableCommand for FullHistory {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let local_registry = ctx.local_registry()?;

        let latest_version = local_registry.get_latest_version();

        info!("Latest version known to the local registry: {latest_version}");

        let full_key = self.full_record_key();

        info!("Will attempt to make full history of key: {full_key}");

        let registry_diagnoser = RegistryDiagnoser { registry: local_registry };

        let chain = registry_diagnoser.fetch_all_changes_for_key_up_to_version(&full_key, latest_version)?;

        info!("Found {} state transitions for queried key", chain.len());

        let (_, agent) = ctx.create_ic_agent_canister_client().await?;

        self.display_chain(chain, RegistryCanisterWrapper::from(agent)).await
    }

    fn validate(&self, _args: &GlobalArgs, cmd: &mut clap::Command) {
        match self.key_type {
            KeyType::SubnetList | KeyType::NodeRewardsTable | KeyType::BlessedReplicaVersions => return,
            KeyType::ApiBoundaryNode
            | KeyType::Node
            | KeyType::NodeOperator
            | KeyType::ReplicaVersion
            | KeyType::HostOsVersion
            | KeyType::Subnet
            | KeyType::DataCenter
                if self.key_value.is_none() => {}
            _ => return,
        }

        cmd.error(
            clap::error::ErrorKind::InvalidValue,
            format!("Value is mandatory with submitted key type"),
        )
        .exit();
    }
}

impl FullHistory {
    fn full_record_key(&self) -> String {
        let prefix = self.key_type.to_registry_prefix();
        match self.key_type {
            KeyType::SubnetList | KeyType::NodeRewardsTable | KeyType::BlessedReplicaVersions => return prefix,
            _ => {}
        }

        // The value has to be here at this point because of the `validate` function
        let value = self.key_value.clone().unwrap();
        format!("{prefix}{value}")
    }

    fn content_to_value(&self, content: RegistryVersionedRecord<Vec<u8>>) -> anyhow::Result<String> {
        let content = match content.value {
            None => return Ok("Deletion Marker".to_string()),
            Some(v) => v,
        };

        let decoded_record = match self.key_type {
            KeyType::SubnetList => SubnetListRecord::decode(content.as_slice()).map(DecodedRecord::SubnetList),
            KeyType::NodeRewardsTable => NodeRewardsTable::decode(content.as_slice()).map(DecodedRecord::NodeRewardsTable),
            KeyType::BlessedReplicaVersions => BlessedReplicaVersions::decode(content.as_slice()).map(DecodedRecord::BlessedReplicaVersions),
            KeyType::ApiBoundaryNode => ApiBoundaryNodeRecord::decode(content.as_slice()).map(DecodedRecord::ApiBoundaryNode),
            KeyType::Node => NodeRecord::decode(content.as_slice()).map(DecodedRecord::Node),
            KeyType::NodeOperator => NodeOperatorRecord::decode(content.as_slice()).map(DecodedRecord::NodeOperator),
            KeyType::ReplicaVersion => ReplicaVersionRecord::decode(content.as_slice()).map(DecodedRecord::ReplicaVersion),
            KeyType::HostOsVersion => HostosVersionRecord::decode(content.as_slice()).map(DecodedRecord::HostOsVersion),
            KeyType::Subnet => SubnetRecord::decode(content.as_slice()).map(DecodedRecord::Subnet),
            KeyType::DataCenter => DataCenterRecord::decode(content.as_slice()).map(DecodedRecord::DataCenter),
        }
        .map_err(anyhow::Error::from)?;

        serialize_decoded_record(decoded_record)
    }

    async fn display_chain<I>(&self, chain: I, reg_canister: RegistryCanisterWrapper) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = RegistryVersionedRecord<Vec<u8>>>,
    {
        let mut mapped_content = vec![];

        for content_at_version in chain {
            let resp = reg_canister
                .get_high_capacity_value(content_at_version.key.clone(), Some(content_at_version.version.get()))
                .await?;

            mapped_content.push(FullRegistryDetail {
                timestamp: match resp.timestamp_nanoseconds {
                    0 => None,
                    nanos => {
                        let duration = std::time::Duration::from_nanos(nanos);
                        std::time::UNIX_EPOCH.checked_add(duration).map(|t| t.into())
                    }
                },
                version: content_at_version.version.get(),
                content: self.content_to_value(content_at_version)?,
            });
        }

        self.display.display_chain(mapped_content);

        Ok(())
    }
}

struct FullRegistryDetail {
    timestamp: Option<DateTime<Utc>>,
    version: u64,
    content: String,
}

#[derive(ValueEnum, Clone, Debug, Display)]
enum DisplayMode {
    Full,
    Diff,
}

impl DisplayMode {
    fn display_chain(&self, chain: Vec<FullRegistryDetail>) {
        match &self {
            DisplayMode::Full => Self::display_full(chain),
            DisplayMode::Diff => Self::display_diff(chain),
        }
    }

    fn display_full(chain: Vec<FullRegistryDetail>) {
        for content_at_version in chain {
            let time = match &content_at_version.timestamp {
                None => "Unknown".to_string(),
                Some(dt) => dt.format("%Y-%m-%d %H:%M:%S%.9f UTC").to_string(),
            };
            println!("Version: {}", content_at_version.version);
            println!("Timestamp: {time}");
            println!("Value:\n{}", content_at_version.content);
            println!();
        }
    }

    fn display_diff(chain: Vec<FullRegistryDetail>) {
        let mut left_content = "".to_string();

        let mut chain_iter = chain.iter();
        let mut maybe_right = chain_iter.next();

        while let Some(right) = maybe_right {
            let diff = TextDiff::from_lines(&left_content, &right.content);

            let time = match &right.timestamp {
                None => "Unknown".to_string(),
                Some(dt) => dt.format("%Y-%m-%d %H:%M:%S%.9f UTC").to_string(),
            };

            // Display
            println!("Version: {}", right.version);
            println!("Timestamp: {time}");
            for change in diff.iter_all_changes() {
                let text = change.to_string();
                let formatted = match change.tag() {
                    similar::ChangeTag::Equal => text.to_string(),
                    similar::ChangeTag::Delete => format!("-{text}").red().to_string(),
                    similar::ChangeTag::Insert => format!("+{text}").bright_green().to_string(),
                };

                print!("{formatted}");
            }
            println!();

            left_content = right.content.clone();
            maybe_right = chain_iter.next();
        }
    }
}
