use std::rc::Rc;

use chrono::Utc;
use ensure_blessed_versions::EnsureBlessedRevisions;
use ic_management_backend::lazy_registry::LazyRegistry;
use itertools::Itertools;
use strum::{EnumIter, IntoEnumIterator};
use tabular_util::{ColumnAlignment, Table};
use upgrade_app_subnets::UpgradeAppSubnets;
use upgrade_deployment_canister::UpgradeDeploymentCanisters;

use crate::ctx::DreContext;

mod ensure_blessed_versions;
mod tabular_util;
mod upgrade_app_subnets;
mod upgrade_deployment_canister;

pub struct QualificationExecutor {
    steps: Vec<Steps>,
}

pub struct QualificationContext {
    dre_ctx: DreContext,
    from_version: String,
    to_version: String,
}

impl QualificationContext {
    pub fn new(dre_ctx: DreContext) -> Self {
        Self {
            dre_ctx,
            from_version: "".to_string(),
            to_version: "".to_string(),
        }
    }

    pub fn with_from_version(self, from_version: String) -> Self {
        Self { from_version, ..self }
    }

    pub fn with_to_version(self, to_version: String) -> Self {
        Self { to_version, ..self }
    }
}

impl QualificationExecutor {
    pub fn with_steps() -> Self {
        Self {
            steps: Steps::iter().collect(),
        }
    }

    pub fn list(&self) {
        let table = Table::new()
            .with_columns(&[("Name", ColumnAlignment::Middle), ("Help", ColumnAlignment::Left)])
            .with_rows(self.steps.iter().map(|s| vec![s.name().to_string(), s.help().to_string()]).collect_vec())
            .to_table();

        println!("{}", table)
    }

    pub async fn execute(&self, ctx: QualificationContext) -> anyhow::Result<()> {
        print_text(format!("Running qualification from version {} to {}", ctx.from_version, ctx.to_version));
        print_text(format!("Starting execution of {} steps:", self.steps.len()));
        for (i, step) in self.steps.iter().enumerate() {
            print_text(format!("Executing step {}: `{}`", i, step.name()));

            step.execute(&ctx).await?;

            print_text(format!("Executed step {}: `{}`", i, step.name()));

            let registry = ctx.dre_ctx.registry().await;
            print_text(format!("Syncing with registry after step {}", i));
            registry.sync_with_nns().await?;

            step.print_status(&ctx).await?
        }

        Ok(())
    }
}

#[derive(EnumIter)]
enum Steps {
    EnsureBlessedVersions(EnsureBlessedRevisions),
    UpgradeDeploymentCanisters(UpgradeDeploymentCanisters),
    UpgradeAppSubnets(UpgradeAppSubnets),
}

pub trait Step {
    fn help(&self) -> &'static str;

    fn name(&self) -> &'static str;

    async fn execute(&self, ctx: &QualificationContext) -> anyhow::Result<()>;

    async fn print_status(&self, ctx: &QualificationContext) -> anyhow::Result<()>;
}

impl Step for Steps {
    fn help(&self) -> &'static str {
        match &self {
            Steps::EnsureBlessedVersions(c) => c.help(),
            Steps::UpgradeDeploymentCanisters(c) => c.help(),
            Steps::UpgradeAppSubnets(c) => c.help(),
        }
    }

    fn name(&self) -> &'static str {
        match &self {
            Steps::EnsureBlessedVersions(c) => c.name(),
            Steps::UpgradeDeploymentCanisters(c) => c.name(),
            Steps::UpgradeAppSubnets(c) => c.name(),
        }
    }

    async fn execute(&self, ctx: &QualificationContext) -> anyhow::Result<()> {
        match &self {
            Steps::EnsureBlessedVersions(c) => c.execute(ctx).await,
            Steps::UpgradeDeploymentCanisters(c) => c.execute(ctx).await,
            Steps::UpgradeAppSubnets(c) => c.execute(ctx).await,
        }
    }

    async fn print_status(&self, ctx: &QualificationContext) -> anyhow::Result<()> {
        match &self {
            Steps::EnsureBlessedVersions(c) => c.print_status(ctx).await,
            Steps::UpgradeDeploymentCanisters(c) => c.print_status(ctx).await,
            Steps::UpgradeAppSubnets(c) => c.print_status(ctx).await,
        }
    }
}

pub async fn print_subnet_versions(registry: Rc<LazyRegistry>) -> anyhow::Result<()> {
    let subnets = registry.subnets().await?;

    let subnets = subnets.values();
    let table = Table::new()
        .with_columns(&[("Subnet Id", ColumnAlignment::Middle), ("Version", ColumnAlignment::Middle)])
        .with_rows(subnets.map(|s| vec![s.principal.to_string(), s.replica_version.clone()]).collect_vec())
        .to_table();

    print_table(table);

    Ok(())
}

pub fn print_text(message: String) {
    _print_with_time(message, false)
}

pub fn print_table(table: tabular::Table) {
    _print_with_time(format!("{}", table), true)
}

fn _print_with_time(message: String, add_new_line: bool) {
    let current_time = Utc::now();

    println!(
        "[{}]{}{}",
        current_time,
        match add_new_line {
            true => '\n',
            false => ' ',
        },
        message
    )
}
