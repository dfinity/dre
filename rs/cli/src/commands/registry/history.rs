use clap::Args;

use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};
use crate::commands::registry::helpers::{validate_range_argument, get_sorted_versions, select_versions, create_writer, flatten_version_records};
use crate::commands::registry::helpers::{Filter, VersionRecord};
use std::path::PathBuf;
use log::info;

#[derive(Args, Debug)]
pub struct History {
    #[clap(allow_hyphen_values = true)]
    pub range: Vec<i64>,

    #[clap(short = 'o', long, help = "Output file (default is stdout)")]
    pub output: Option<PathBuf>,

    #[clap(long, short, alias = "filter", help = Filter::get_help_message())]
    pub filter: Vec<Filter>,
}

impl ExecutableCommand for History {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        use ic_registry_common_proto::pb::local_store::v1::ChangelogEntry as PbChangelogEntry;

        // Resolve range
        let validated_range = validate_range_argument(&self.range)?;
        let (versions_sorted, entries_sorted) = get_sorted_versions(&ctx).await?;
        let range = if validated_range.is_empty() { None } else { Some(validated_range) };
        let selected_versions = select_versions(range, &versions_sorted)?;

        // Log versions
        info!("Selected version range from {} to {}", selected_versions.first().unwrap(), selected_versions.last().unwrap());

        // Build flat list of records
        let entries_map: std::collections::HashMap<u64, PbChangelogEntry> = entries_sorted.into_iter().collect();
        let out: Vec<VersionRecord> = flatten_version_records(&selected_versions, &entries_map);

        // Write to file or stdout
        let writer = create_writer(&self.output)?;
        serde_json::to_writer_pretty(writer, &out)?;

        Ok(())
    }
}
