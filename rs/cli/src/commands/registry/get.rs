use clap::Args;

use crate::commands::registry::helpers::versions::{VersionRange, VersionFillMode};
use crate::commands::registry::helpers::dump::{get_sorted_versions_from_local, get_dump_from_registry};
use crate::commands::registry::helpers::filters::Filter;
use crate::commands::registry::helpers::writer::create_writer;
use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};
use std::path::PathBuf;
use log::info;

#[derive(Args, Debug)]
#[clap(about = "Get aggregated registry data for a specific version.

Version numbers:
  - Positive numbers are actual version numbers
  - Negative numbers are indices relative to the latest version (-1 = latest)
  - 0 is not supported
  - No argument will show history of the latest version

Examples:
  -5              # Get data of latest-5
  -1              # Get data of latest version
  55400           # Get data of version 55400
")]
pub struct Get {
    #[clap(index = 1, allow_hyphen_values = true, help = "Version number or negative index")]
    pub version: Option<i64>,

    #[clap(short = 'o', long, help = "Output file (default is stdout)")]
    pub output: Option<PathBuf>,

    #[clap(long, short, alias = "filter", help = Filter::get_help_message())]
    pub filter: Vec<Filter>,
}

impl ExecutableCommand for Get {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        // Ensure local registry is initialized/synced
        let _ = ctx.load_registry_for_version(None).await;

        // Get sorted versions
        let (versions_in_registry, _) = get_sorted_versions_from_local(&ctx).await?;

        // Create version range
        let version_range = VersionRange::create_from_args(self.version, None, VersionFillMode::FromStart, &versions_in_registry)?;
        info!("Selected version range: {:?}", version_range);

        // Clear registry cache and fetch specific version if version is not None
        if self.version.is_some() {
            ctx.clear_registry_cache();
            let _ = ctx.load_registry_for_version(version_range.get_to()).await;
        }

        // Get registry dump
        let registry_dump = get_dump_from_registry(ctx).await?;
        let mut serde_value = serde_json::to_value(registry_dump)?;

        // Apply filters
        self.filter.iter().for_each(|filter| {
            let _ = filter.filter_json_value(&mut serde_value);
        });

        // Write to file or stdout
        let writer = create_writer(&self.output)?;
        serde_json::to_writer_pretty(writer, &serde_value)?;

        Ok(())
    }
}
