use clap::Args;

use crate::commands::registry::helpers::versions::{VersionRange, VersionFillMode};
use crate::commands::registry::helpers::dump::{get_sorted_versions_from_local, get_dump_from_registry};
use crate::commands::registry::helpers::filters::Filter;
use crate::commands::registry::helpers::writer::Writer;
use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};
use std::path::PathBuf;
use log::info;

#[derive(Args, Debug)]
#[clap(about = format!("Get aggregated data for a specific version"))]

pub struct Get {
    #[clap(index = 1, allow_hyphen_values = true, help = format!("Version number or negative index

- No argument will get data of the latest version.
{}

Examples:
  -5              # Get data of latest-5
  -1              # Get data of latest version
  55400           # Get data of version 55400
", VersionRange::get_help_text()))]

    pub version: Option<i64>,

    #[clap(short = 'o', long, help = "Output file (default is stdout)")]
    pub output: Option<PathBuf>,

    #[clap(short = 'f', long, help = Filter::get_help_text())]
    pub filter: Vec<Filter>,
}

impl ExecutableCommand for Get {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, mut ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        // Ensure local registry is initialized/synced
        let _ = ctx.load_registry().await;

        // Get sorted versions
        let (versions_sorted, _) = get_sorted_versions_from_local(&ctx).await?;

        // Create version range
        let version_range = VersionRange::create_from_args(self.version, None, VersionFillMode::FromStart, &versions_sorted)?;
        info!("Selected version range {:?}", version_range);

        // Clear registry cache and fetch specific version if version is not None
        if self.version.is_some() {
            ctx.clear_registry_cache();
            let _ = ctx.load_registry_for_version(Some(version_range.get_to())).await;
        }

        // Get registry dump
        let registry_dump = get_dump_from_registry(ctx).await?;
        let mut serde_value = serde_json::to_value(registry_dump)?;

        // Apply filters
        self.filter.iter().for_each(|filter| {
            let _ = filter.filter_json_value(&mut serde_value);
        });

        // Write to file or stdout
        let mut writer = Writer::new(&self.output, false)?;
        writer.write_line(&serde_json::to_string_pretty(&serde_value)?)?;

        Ok(())
    }
}
