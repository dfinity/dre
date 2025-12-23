use clap::Args;

use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};
use std::path::PathBuf;
use crate::commands::registry::helpers::filters::Filter;

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
  55400           # Show diff between version 1 and 100
  -5 -2           # Show diff between latest-5 and latest-2
  55400 55440     # Show diff between version 10 and 15
  ")]
pub struct Diff {
    #[clap(index = 1, allow_hyphen_values = true, help = "Version in range (optional)")]
    pub version_1: Option<i64>,

    #[clap(index = 2, allow_hyphen_values = true, help = "Version in range (optional)")]
    pub version_2: Option<i64>,

    #[clap(short = 'o', long, help = "Output file (default is stdout)")]
    pub output: Option<PathBuf>,

    #[clap(long, short, alias = "filter", help = Filter::get_help_message())]
    pub filter: Vec<Filter>,
}

impl ExecutableCommand for Diff {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        // Build range vector from optional version_1/version_2 fields
        // let range: Vec<i64> = match (self.version_1, self.version_2) {
        //     (Some(f), Some(t)) => vec![f, t],
        //     (Some(f), None) => vec![f],
        //     (None, None) => vec![],
        //     (None, Some(_)) => anyhow::bail!("This should never happen"),
        // };

        // // Resolve range
        // let validated_range = validate_range_argument(&range)?;
        // let range = if validated_range.is_empty() { None } else { Some(validated_range) };
        // let (versions_sorted, _) = get_sorted_versions(&ctx).await?;
        // let selected_versions = select_versions(range, &versions_sorted)?;

        // // Take first and last from selected versions
        // let actual_v1 = *selected_versions.first().unwrap();
        // let actual_v2 = *selected_versions.last().unwrap();

        // // Log versions
        // info!("Selected version range from {} to {}", actual_v1, actual_v2,);

        // // Fetch aggregated registry data for both versions
        // // Clear registry cache before each call to ensure we get the correct version
        // ctx.clear_registry_cache();
        // let reg1 = get_registry(ctx.clone(), Some(actual_v1)).await?;
        // ctx.clear_registry_cache();
        // let reg2 = get_registry(ctx.clone(), Some(actual_v2)).await?;

        // // Apply filters
        // let mut val1 = serde_json::to_value(&reg1)?;
        // let mut val2 = serde_json::to_value(&reg2)?;
        // self.filter.iter().for_each(|filter| {
        //     let _ = filter.filter_json_value(&mut val1);
        //     let _ = filter.filter_json_value(&mut val2);
        // });

        // // Create diff: val2 - val1
        // let json1 = serde_json::to_string_pretty(&val1)?;
        // let json2 = serde_json::to_string_pretty(&val2)?;
        // let diff = TextDiff::from_lines(&json2, &json1);

        // // Write to file or stdout
        // let mut writer = create_writer(&self.output)?;

        // // Use color if output is stdout
        // let use_color = self.output.is_none() && std::io::IsTerminal::is_terminal(&std::io::stdout());

        // // Write diff to file or stdout
        // for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        //     if idx > 0 {
        //         writeln!(writer, "{}", "---".dimmed())?;
        //     }
        //     for op in group {
        //         for change in diff.iter_changes(op) {
        //             let (sign, s) = match change.tag() {
        //                 ChangeTag::Delete => ("-", change.to_string()),
        //                 ChangeTag::Insert => ("+", change.to_string()),
        //                 ChangeTag::Equal => (" ", change.to_string()),
        //             };
        //             if use_color {
        //                 let color = match change.tag() {
        //                     ChangeTag::Delete => colored::Color::Red,
        //                     ChangeTag::Insert => colored::Color::Green,
        //                     ChangeTag::Equal => colored::Color::White,
        //                 };
        //                 write!(writer, "{}{} ", sign.color(color), s.color(color))?;
        //             } else {
        //                 write!(writer, "{}{} ", sign, s)?;
        //             }
        //         }
        //     }
        // }

        println!("diff");

        Ok(())
    }
}
