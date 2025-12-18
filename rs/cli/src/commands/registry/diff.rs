use clap::Args;

use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};
use crate::commands::registry::helpers::{validate_range_argument, get_sorted_versions, select_versions, create_writer, filter_json_value, get_registry};
use crate::commands::registry::helpers::Filter;
use colored::Colorize;
use similar::TextDiff;
use similar::ChangeTag;
use std::path::PathBuf;
use log::info;

#[derive(Args, Debug)]
#[clap(about = "Show diff of two aggregated versions of the registry.

Version numbers:
  - Positive numbers are actual version numbers
  - Negative numbers are indices relative to the latest version (-1 = latest)
  - 0 is not supported
  - No argument will show diff between latest and latest-10
  - Version numbers are inclusive.

Examples:
  -5              # Show diff between latest-5 and latest
  100             # Show diff between version 1 and 100
  -5 -2           # Show diff between latest-5 and latest-2
  10 15           # Show diff between version 10 and 15
  ")]
pub struct Diff{
    #[clap(index = 1, allow_hyphen_values = true, help = "Version in range (optional)")]
    pub version_1: Option<i64>,

    #[clap(index = 2, allow_hyphen_values = true, help = "Version in range (optional)")]
    pub version_2: Option<i64>,

    #[clap(short = 'o', long, help = "Output file (default is stdout)")]
    pub output: Option<PathBuf>,

    #[clap(long, short, alias = "filter", help = Filter::get_help_message())]
    pub filters: Vec<Filter>,
}

impl ExecutableCommand for Diff {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        // Build range vector from optional version_1/version_2 fields
        let range: Vec<i64> = match (self.version_1, self.version_2) {
            (Some(f), Some(t)) => vec![f, t],
            (Some(f), None) => vec![f],
            (None, None) => vec![],
            (None, Some(_)) => anyhow::bail!("This should never happen"),
        };

        // Resolve range
        let validated_range = validate_range_argument(&range)?;
        let range = if validated_range.is_empty() { None } else { Some(validated_range) };
        let (versions_sorted, _) = get_sorted_versions(&ctx).await?;
        let selected_versions = select_versions(range, &versions_sorted)?;

        // Log versions
        info!("Selected version range from {} to {}", selected_versions.first().unwrap(), selected_versions.last().unwrap());

        // Take first and last from selected versions
        let actual_v1 = selected_versions[0];
        let actual_v2 = *selected_versions.last().unwrap();

        // Fetch registry for higher version first (online/sync), then lower version (offline)
        let (reg1, reg2) = {
            if actual_v1 > actual_v2 {
                let r1 = get_registry(ctx.clone(), Some(actual_v1), false).await?;
                let r2 = get_registry(ctx.clone(), Some(actual_v2), true).await?;
                (r1, r2)
            } else {
                let r2 = get_registry(ctx.clone(), Some(actual_v2), false).await?;
                let r1 = get_registry(ctx.clone(), Some(actual_v1), true).await?;
                (r1, r2)
            }
        };

        // Apply filters
        let mut val1 = serde_json::to_value(&reg1)?;
        let mut val2 = serde_json::to_value(&reg2)?;
        self.filters.clone().iter().for_each(|filter| {
            let _ = filter_json_value(&mut val1, &filter.key.clone(), &filter.value.clone(), &filter.comparison.clone());
            let _ = filter_json_value(&mut val2, &filter.key.clone(), &filter.value.clone(), &filter.comparison.clone());
        });

        // Create diff
        let json1 = serde_json::to_string_pretty(&val1)?;
        let json2 = serde_json::to_string_pretty(&val2)?;
        let diff = TextDiff::from_lines(&json1, &json2);

        // Write to file or stdout
        let mut writer = create_writer(&self.output)?;

        // Use color if output is stdout
        let use_color = self.output.is_none() && std::io::IsTerminal::is_terminal(&std::io::stdout());

        // Write diff to file or stdout
        for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
            if idx > 0 {
                writeln!(writer, "{}", "---".dimmed().to_string())?;
            }
            for op in group {
                for change in diff.iter_changes(op) {
                    let (sign, s) = match change.tag() {
                        ChangeTag::Delete => ("-", change.to_string()),
                        ChangeTag::Insert => ("+", change.to_string()),
                        ChangeTag::Equal => (" ", change.to_string()),
                    };
                    if use_color {
                        let color = match change.tag() {
                            ChangeTag::Delete => colored::Color::Red,
                            ChangeTag::Insert => colored::Color::Green,
                            ChangeTag::Equal => colored::Color::White,
                        };
                        write!(writer, "{}{} ", sign.color(color), s.color(color))?;
                    } else {
                        write!(writer, "{}{} ", sign, s)?;
                    }
                }
            }
        }

        Ok(())
    }
}
