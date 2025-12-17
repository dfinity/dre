use clap::Args;

use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};
use crate::commands::registry::helpers::{validate_range, get_sorted_versions, select_versions, filter_json_value, get_registry, create_writer};
use crate::commands::registry::helpers::Filter;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct Get {
    #[clap(allow_hyphen_values = true)]
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
        // Resolve version: if negative, use select_versions with range from input to -1, then take first element
        let height: Option<u64> = if let Some(h) = self.version {
            if h < 0 {
                // Negative: create range vector, validate it, then use select_versions
                let range = vec![h, -1];
                let validated_range = validate_range(&range)?;
                let (versions_sorted, _) = get_sorted_versions(&ctx).await?;
                let range_opt = if validated_range.is_empty() { None } else { Some(validated_range) };
                let selected = select_versions(range_opt, &versions_sorted)?;
                selected.first().copied()
            } else {
                Some(h as u64)
            }
        } else {
            None
        };

        // Aggregated registry view
        let registry = get_registry(ctx, height, false).await?;
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
