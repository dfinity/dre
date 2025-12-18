use clap::Args;

use crate::commands::registry::helpers::{Filter, VersionRecord};
use crate::commands::registry::helpers::{create_writer, flatten_version_records, get_sorted_versions, select_versions, validate_range_argument};
use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};
use log::info;
use std::path::PathBuf;

#[derive(Args, Debug)]
#[clap(about = "Show registry history for a version range.

Version numbers:
  - Positive numbers are actual version numbers
  - Negative numbers are indices relative to the latest version (-1 = latest)
  - 0 is not supported
  - No argument will show history of the latest 10 versions
  - Version numbers are inclusive.

Examples:
  -5              # Show history from latest-5 to latest
  -1              # Show history of latest version only
  55400           # Show history from version 1 to 100
  -5 -2           # Show history from latest-5 to latest-2
  55400 55440     # Show history from version 10 to 15
  ")]
pub struct History {
    #[clap(index = 1, allow_hyphen_values = true, help = "Version in range (optional)")]
    pub version_1: Option<i64>,

    #[clap(index = 2, allow_hyphen_values = true, help = "Version in range (optional)")]
    pub version_2: Option<i64>,

    #[clap(short = 'o', long, help = "Output file (default is stdout)")]
    pub output: Option<PathBuf>,

    #[clap(short = 'f', long, help = Filter::get_help_message())]
    pub filter: Vec<Filter>,
}

impl ExecutableCommand for History {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        use ic_registry_common_proto::pb::local_store::v1::ChangelogEntry as PbChangelogEntry;

        // Build range vector from optional version_1/version_2 fields
        let range: Vec<i64> = match (self.version_1, self.version_2) {
            (Some(f), Some(t)) => vec![f, t],
            (Some(f), None) => vec![f],
            (None, None) => vec![],
            (None, Some(_)) => anyhow::bail!("This should never happen"),
        };

        // Resolve range
        let validated_range = validate_range_argument(&range)?;
        let (versions_sorted, entries_sorted) = get_sorted_versions(&ctx).await?;
        let range = if validated_range.is_empty() { None } else { Some(validated_range) };
        let selected_versions = select_versions(range, &versions_sorted)?;

        // Log versions
        info!(
            "Selected version range from {} to {}",
            selected_versions.first().unwrap(),
            selected_versions.last().unwrap()
        );

        // Build flat list of records
        let entries_map: std::collections::HashMap<u64, PbChangelogEntry> = entries_sorted.into_iter().collect();
        let out: Vec<VersionRecord> = flatten_version_records(&selected_versions, &entries_map);

        // Write to file or stdout
        let writer = create_writer(&self.output)?;
        serde_json::to_writer_pretty(writer, &out)?;

        Ok(())
    }
}
