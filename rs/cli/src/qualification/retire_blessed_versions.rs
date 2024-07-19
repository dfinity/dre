use itertools::Itertools;

use crate::{
    ctx::DreContext,
    ic_admin::{ProposeCommand, ProposeOptions},
};

use super::{
    ic_admin_with_retry, print_table, print_text,
    tabular_util::{ColumnAlignment, Table},
    Step,
};

pub struct RetireBlessedVersions {
    pub versions: Vec<String>,
}

impl Step for RetireBlessedVersions {
    fn help(&self) -> String {
        "Ensure that versions are retired".to_string()
    }

    fn name(&self) -> String {
        "retire_blessed_versions".to_string()
    }

    async fn execute(&self, ctx: &DreContext) -> anyhow::Result<()> {
        let registry = ctx.registry().await;

        let blessed_versions = registry.elected_guestos()?;
        let mut to_unelect = vec![];
        for version in &self.versions {
            if blessed_versions.contains(version) {
                to_unelect.push(version);
            }
        }
        if to_unelect.is_empty() {
            print_text(format!("Versions {} are not blessed, skipping step", self.versions.iter().join(",")));
            return Ok(());
        }

        ic_admin_with_retry(
            ctx.ic_admin(),
            ProposeCommand::ReviseElectedVersions {
                release_artifact: ic_management_types::Artifact::GuestOs,
                args: to_unelect
                    .iter()
                    .flat_map(|v| vec!["--replica-versions-to-unelect".to_string(), v.to_string()])
                    .collect(),
            },
            ProposeOptions {
                title: Some("Retire replica versions".to_string()),
                summary: Some("Unelecting a version".to_string()),
                motivation: Some("Unelecting a version".to_string()),
            },
        )
        .await?;

        registry.sync_with_nns().await?;
        let blessed_versions = registry.elected_guestos()?;

        let table = Table::new()
            .with_columns(&[("Blessed versions", ColumnAlignment::Middle)])
            .with_rows(blessed_versions.iter().map(|ver| vec![ver.to_string()]).collect_vec())
            .to_table();

        print_table(table);

        Ok(())
    }
}
