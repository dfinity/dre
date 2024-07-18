use itertools::Itertools;

use crate::{
    ic_admin::{ProposeCommand, ProposeOptions},
    qualification::Step,
};

use super::{
    print_table, print_text,
    tabular_util::{ColumnAlignment, Table},
    QualificationContext,
};

#[derive(Default)]
pub struct EnsureBlessedRevisions {}

impl Step for EnsureBlessedRevisions {
    fn help(&self) -> &'static str {
        "This step runs the check to determine if all versions are blessed"
    }

    fn name(&self) -> &'static str {
        "1a_ensure_blessed_revision"
    }

    async fn execute(&self, ctx: &QualificationContext) -> anyhow::Result<()> {
        let registry = ctx.dre_ctx.registry().await;
        let blessed_versions = registry.elected_guestos()?;

        if blessed_versions.contains(&ctx.to_version) {
            return Ok(());
        }

        // Place proposal
        let ic_admin = ctx.dre_ctx.ic_admin();
        ic_admin.propose_run(
            ProposeCommand::ReviseElectedVersions {
                release_artifact: ic_management_types::Artifact::GuestOs,
                args: vec![],
            },
            ProposeOptions {
                title: Some(format!("Blessing version: {}", &ctx.to_version)),
                summary: Some(format!("Some updates")),
                ..Default::default()
            },
        );
        // Vote

        Ok(())
    }

    async fn print_status(&self, ctx: &QualificationContext) -> anyhow::Result<()> {
        let registry = ctx.dre_ctx.registry().await;
        let blessed_versions = registry.elected_guestos()?;

        let table = Table::new()
            .with_columns(&[("Blessed versions", ColumnAlignment::Middle)])
            .with_rows(blessed_versions.iter().map(|ver| vec![ver.to_string()]).collect_vec())
            .to_table();

        print_table(table);

        Ok(())
    }
}
