use clap::Args;

use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};
use crate::commands::registry::helpers::{validate_range_argument, get_sorted_versions, select_versions, filter_json_value, get_registry, create_writer};
use crate::commands::registry::helpers::Filter;
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
  100             # Get data of version 100
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
        // Resolve version
        let (version, registry_has_been_syned): (Option<u64>, bool) = if let Some(h) = self.version {
            if h < 0 {
                // Negative: find version based on relative index
                let range = vec![h, -1];
                let validated_range = validate_range_argument(&range)?;
                let (versions_sorted, _) = get_sorted_versions(&ctx).await?;
                let range_opt = if validated_range.is_empty() { None } else { Some(validated_range) };
                let selected = select_versions(range_opt, &versions_sorted)?;
                (selected.first().copied(), true)
            } else {
                // Positive: return the version number as is
                (Some(h as u64), false)
            }
        } else {
            // No version provided: return None for latest version
            (None, false)
        };

        // Log version
        if let Some(version) = version {
            info!("Selected version {}", version);
        } else {
            info!("Selected latest version");
        }

        // Aggregated registry view. Only sync if the registry has not been synced yet.
        let registry = get_registry(ctx, version, registry_has_been_syned).await?;
        let mut serde_value = serde_json::to_value(registry)?;

        // Apply filters
        self.filter.clone().iter().for_each(|filter| {
            let _ = filter_json_value(&mut serde_value, &filter.key.clone(), &filter.value.clone(), &filter.comparison.clone());
        });

        // Write to file or stdout
        let writer = create_writer(&self.output)?;
        serde_json::to_writer_pretty(writer, &serde_value)?;

        Ok(())
    }
}
