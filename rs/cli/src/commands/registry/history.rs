use clap::Args;

use crate::commands::registry::helpers::filters::Filter;
use crate::commands::registry::helpers::versions::VersionRange;
use crate::commands::registry::helpers::dump::get_sorted_versions_from_local;
use crate::commands::registry::helpers::versions::VersionFillMode;
use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};
use std::path::PathBuf;
use log::info;

#[derive(Args, Debug)]
#[clap(about = "Show history for a version range")]
pub struct History {
    #[clap(index = 1, allow_hyphen_values = true, help = format!("Version number or negative index

- No argument will show history from latest-10 to latest
{}

Examples:
  -5              # Show history of latest-5 to latest
  -1              # Show history of latest
  55400           # Show history from 55400 to latest
", VersionRange::get_help_text()))]
    pub version_1: Option<i64>,

    #[clap(index = 2, allow_hyphen_values = true, help = "Version number or negative index

See [VERSION_1] for more information.
Only supported in combination with [VERSION_1].

Examples for combination with [VERSION_1]:
  -5 -2           # Show history of latest-5 to latest-2
  55400 55450     # Show history from 55400 to 55450
    ")]
    pub version_2: Option<i64>,

    #[clap(short = 'o', long, help = "Output file (default is stdout)")]
    pub output: Option<PathBuf>,

    #[clap(short = 'f', long, help = Filter::get_help_text())]
    pub filter: Vec<Filter>,
}

impl ExecutableCommand for History {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        // Ensure local registry is initialized/synced
        let _ = ctx.load_registry().await;

        // Get sorted versions
        let (versions_in_registry, _) = get_sorted_versions_from_local(&ctx).await?;

        // Create version range
        let version_range = VersionRange::create_from_args(self.version_1, self.version_2, VersionFillMode::ToEnd, &versions_in_registry)?;
        info!("Selected version range: {:?}", version_range);

        // // Build flat list of records
        // let entries_map: std::collections::HashMap<u64, PbChangelogEntry> = entries_sorted.into_iter().collect();
        // let out: Vec<VersionRecord> = flatten_version_records(&selected_versions, &entries_map);

        // // Write to file or stdout
        // let writer = create_writer(&self.output)?;
        // serde_json::to_writer_pretty(writer, &out)?;

        Ok(())
    }
}
