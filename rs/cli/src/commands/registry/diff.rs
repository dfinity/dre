use crate::commands::registry::helpers::dump::RegistryDump;
use crate::commands::registry::helpers::dump::{get_dump_from_registry, get_sorted_versions_from_local};
use crate::commands::registry::helpers::filters::Filter;
use crate::commands::registry::helpers::versions::{VersionFillMode, VersionRange};
use crate::commands::registry::helpers::writer::Writer;
use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};
use clap::Args;
use colored::Colorize;
use log::info;
use similar::TextDiff;
use std::path::PathBuf;

#[derive(Args, Debug)]
#[clap(about = "Show diff of the data between two aggregated versions")]
pub struct Diff {
    #[clap(index = 1, allow_hyphen_values = true, help = format!("Version number or negative index

- No argument will show diff from latest-10 to latest
{}

Examples:
  -5              # Show diff of latest-5 to latest
  -1              # Show diff of latest
  55400           # Show diff from 55400 to latest
", VersionRange::get_help_text()))]
    pub version_1: Option<i64>,

    #[clap(
        index = 2,
        allow_hyphen_values = true,
        help = "Version number or negative index

See [VERSION_1] for more information.
Only supported in combination with [VERSION_1].

Examples for combination with [VERSION_1]:
  -5 -2           # Show diff of latest-5 to latest-2
  55400 55450     # Show diff from 55400 to 55450
    "
    )]
    pub version_2: Option<i64>,

    #[clap(short = 'o', long, help = "Output file (default is stdout)")]
    pub output: Option<PathBuf>,

    #[clap(short = 'f', long, help = Filter::get_help_text())]
    pub filter: Vec<Filter>,
}

impl ExecutableCommand for Diff {
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
        let version_range = VersionRange::create_from_args(self.version_1, self.version_2, VersionFillMode::ToEnd, &versions_sorted)?;
        info!("Selected version range {:?}", version_range);

        // Fetch data for version 1
        ctx.clear_registry_cache();
        let _ = ctx.load_registry_for_version(Some(version_range.get_to())).await;
        let registry_dump_v1: RegistryDump = get_dump_from_registry(ctx.clone()).await?;
        let mut serde_value_v1 = serde_json::to_value(registry_dump_v1)?;

        // Apply filters to version 1
        self.filter.iter().for_each(|filter| {
            let _ = filter.filter_json_value(&mut serde_value_v1);
        });

        // Fetch data for version 2
        ctx.clear_registry_cache();
        let _ = ctx.load_registry_for_version(Some(version_range.get_from())).await;
        let registry_dump_v2: RegistryDump = get_dump_from_registry(ctx.clone()).await?;
        let mut serde_value_v2 = serde_json::to_value(registry_dump_v2)?;

        // Apply filters to version 2
        self.filter.iter().for_each(|filter| {
            let _ = filter.filter_json_value(&mut serde_value_v2);
        });

        // Create diff: v2 - v1
        let json1 = serde_json::to_string_pretty(&serde_value_v1)?;
        let json2 = serde_json::to_string_pretty(&serde_value_v2)?;
        let diff = TextDiff::from_lines(&json2, &json1);

        // Use color if output is stdout
        let use_color = self.output.is_none() && std::io::IsTerminal::is_terminal(&std::io::stdout());

        // Create writer
        let mut writer = Writer::new(&self.output, use_color)?;

        // Write diff to file or stdout
        generate_diff(&diff, &mut writer)?;

        Ok(())
    }
}

fn generate_diff(diff: &TextDiff<'_, '_, '_, str>, writer: &mut Writer) -> anyhow::Result<()> {
    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            writer.write_line(&"---".dimmed())?;
        }
        for op in group {
            for change in diff.iter_changes(op) {
                writer.write_diff_line(&change.tag(), &change.to_string())?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Writer, generate_diff};
    use similar::TextDiff;
    use std::path::PathBuf;

    #[test]
    fn test_generate_diff() {
        // Test data
        let json1 = serde_json::json!({
            "a": 1,
            "b": 2
        });
        let json2 = serde_json::json!({
            "a": 1,
            "b": 3
        });

        // Generate diff
        let json1_str: String = serde_json::to_string_pretty(&json1).unwrap();
        let json2_str: String = serde_json::to_string_pretty(&json2).unwrap();
        let diff = TextDiff::from_lines(json1_str.as_str(), json2_str.as_str());

        // Write diff to file
        let mut writer = Writer::new(&Some(PathBuf::from("/tmp/diff_test_output.json")), false).unwrap();
        generate_diff(&diff, &mut writer).unwrap();
        // Flush data to disk
        writer.flush().unwrap(); // Ensure data is written to disk
        drop(writer); // Explicitly drop to ensure file is closed

        // Read diff output from file
        let diff_output = fs_err::read_to_string("/tmp/diff_test_output.json").unwrap();

        // Assert diff output contains expected changes
        assert!(diff_output.contains("  \"a\": 1,"));
        assert!(diff_output.contains("-  \"b\": 2"));
        assert!(diff_output.contains("+  \"b\": 3"));

        // cleanup
        fs_err::remove_file("/tmp/diff_test_output.json").unwrap();
    }
}
