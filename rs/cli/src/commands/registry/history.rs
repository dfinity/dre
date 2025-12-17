use clap::Args;

use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};
use crate::commands::registry::helpers::{validate_range, get_sorted_versions, select_versions, create_writer, flatten_version_records};
use crate::commands::registry::helpers::Filter;
use crate::commands::registry::helpers::VersionRecord;
use std::path::PathBuf;
use log::info;

#[derive(Args, Debug)]
pub struct History {
    #[clap(allow_hyphen_values = true)]
    pub range: Vec<i64>,

    pub output: Option<PathBuf>,

    pub filters: Vec<Filter>,
}

impl ExecutableCommand for History {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        use ic_registry_common_proto::pb::local_store::v1::ChangelogEntry as PbChangelogEntry;

        let validated_range = validate_range(&self.range)?;
        let (versions_sorted, entries_sorted) = get_sorted_versions(&ctx).await?;
        let range = if validated_range.is_empty() { None } else { Some(validated_range) };

        let selected_versions = select_versions(range, &versions_sorted)?;
        if let (Some(&first), Some(&last)) = (selected_versions.first(), selected_versions.last()) {
            if first == last {
                info!("Selected version {}", first);
            } else {
                info!("Selected versions from {} to {}", first, last);
            }
        }

        // Build flat list of records
        let entries_map: std::collections::HashMap<u64, PbChangelogEntry> = entries_sorted.into_iter().collect();
        let out: Vec<VersionRecord> = flatten_version_records(&selected_versions, &entries_map);

        // Write to file or stdout
        let writer = create_writer(&self.output)?;
        serde_json::to_writer_pretty(writer, &out)?;

        Ok(())
    }
}
