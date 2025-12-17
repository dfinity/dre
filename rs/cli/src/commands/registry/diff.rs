use clap::Args;

use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};
use crate::commands::registry::helpers::{validate_range, get_sorted_versions, select_versions, create_writer, filter_json_value, get_registry};
use crate::commands::registry::helpers::Filter;
use colored::Colorize;
use similar::TextDiff;
use similar::ChangeTag;
use std::path::PathBuf;
use log::info;

#[derive(Args, Debug)]
pub struct Diff{
    #[clap(allow_hyphen_values = true)]
    pub range: Vec<i64>,

    pub output: Option<PathBuf>,

    pub filters: Vec<Filter>,
}

impl ExecutableCommand for Diff {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let validated_range = validate_range(&self.range)?;
        let range = if validated_range.is_empty() { None } else { Some(validated_range) };
        let (versions_sorted, _) = get_sorted_versions(&ctx).await?;

        let selected_versions = select_versions(range, &versions_sorted)?;
        if let (Some(&first), Some(&last)) = (selected_versions.first(), selected_versions.last()) {
            if first == last {
                info!("Selected version {}", first);
            } else {
                info!("Selected versions from {} to {}", first, last);
            }
        }

        // Take first and last from selected versions
        let actual_v1 = selected_versions[0];
        let actual_v2 = *selected_versions.last().unwrap();

        let (reg1, reg2) = {
            // Logic: Fetch High version (online/sync) first, then Low version (offline)
            if actual_v1 > actual_v2 {
                let r1 = get_registry(ctx.clone(), Some(actual_v1), false).await?;
                let r2 = get_registry(ctx.clone(), Some(actual_v2), true).await?;
                (r1, r2)
            } else {
                // actual_v2 >= actual_v1. Fetch High (v2) first (sync), then Low (v1) offline.
                let r2 = get_registry(ctx.clone(), Some(actual_v2), false).await?;
                let r1 = get_registry(ctx.clone(), Some(actual_v1), true).await?;
                (r1, r2)
            }
        };

        let mut val1 = serde_json::to_value(&reg1)?;
        let mut val2 = serde_json::to_value(&reg2)?;

        self.filters.clone().iter().for_each(|filter| {
            let _ = filter_json_value(&mut val1, &filter.key.clone(), &filter.value.clone(), &filter.comparison.clone());
            let _ = filter_json_value(&mut val2, &filter.key.clone(), &filter.value.clone(), &filter.comparison.clone());
        });

        let json1 = serde_json::to_string_pretty(&val1)?;
        let json2 = serde_json::to_string_pretty(&val2)?;

        let diff = TextDiff::from_lines(&json1, &json2);

        // Write to file or stdout
        let mut writer = create_writer(&self.output)?;

        let use_color = self.output.is_none() && std::io::IsTerminal::is_terminal(&std::io::stdout());

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
