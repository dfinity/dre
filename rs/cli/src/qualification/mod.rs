use std::{rc::Rc, sync::Arc};

use chrono::Utc;
use ensure_blessed_versions::EnsureBlessedRevisions;
use ic_management_backend::lazy_registry::LazyRegistry;
use ic_registry_subnet_type::SubnetType;
use itertools::Itertools;
use retire_blessed_versions::RetireBlessedVersions;
use tabular_util::{ColumnAlignment, Table};
use upgrade_deployment_canister::UpgradeDeploymentCanisters;
use upgrade_subnets::{Action, UpgradeSubnets};

use crate::{
    ctx::DreContext,
    ic_admin::{IcAdminWrapper, ProposeCommand, ProposeOptions},
};

mod ensure_blessed_versions;
mod retire_blessed_versions;
mod tabular_util;
mod upgrade_deployment_canister;
mod upgrade_subnets;

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
    pub fn new(ctx: &QualificationContext) -> Self {
        Self {
            steps: vec![
                // Blessing the version which we are qualifying
                Steps::EnsureBlessedVersions(EnsureBlessedRevisions {
                    version: ctx.to_version.clone(),
                }),
                // Upgrading deployment canisters
                Steps::UpgradeDeploymentCanisters(UpgradeDeploymentCanisters {}),
                // Upgrading all application subnets
                Steps::UpgradeSubnets(UpgradeSubnets {
                    action: Action::Upgrade,
                    subnet_type: Some(SubnetType::Application),
                    to_version: ctx.to_version.clone(),
                }),
                // Upgrading all system subnets
                Steps::UpgradeSubnets(UpgradeSubnets {
                    action: Action::Upgrade,
                    subnet_type: Some(SubnetType::System),
                    to_version: ctx.to_version.clone(),
                }),
                // Upgrading unassigned nodes
                Steps::UpgradeSubnets(UpgradeSubnets {
                    action: Action::Upgrade,
                    subnet_type: None,
                    to_version: ctx.to_version.clone(),
                }),
                // Since the initial testnet is spunup with disk-img
                // retire the initial version.
                Steps::RetireBlessedVersions(RetireBlessedVersions {
                    versions: vec![ctx.from_version.clone()],
                }),
                // Bless initial replica version with update-img
                Steps::EnsureBlessedVersions(EnsureBlessedRevisions {
                    version: ctx.from_version.clone(),
                }),
                // Downgrade application subnets
                Steps::UpgradeSubnets(UpgradeSubnets {
                    action: Action::Downgrade,
                    subnet_type: Some(SubnetType::Application),
                    to_version: ctx.from_version.clone(),
                }),
                // Downgrade system subnets
                Steps::UpgradeSubnets(UpgradeSubnets {
                    action: Action::Downgrade,
                    subnet_type: Some(SubnetType::System),
                    to_version: ctx.from_version.clone(),
                }),
                // Downgrade unassinged nodes
                Steps::UpgradeSubnets(UpgradeSubnets {
                    action: Action::Downgrade,
                    subnet_type: None,
                    to_version: ctx.from_version.clone(),
                }),
            ],
        }
    }

    pub fn list(&self) {
        let table = Table::new()
            .with_columns(&[("Name", ColumnAlignment::Left), ("Help", ColumnAlignment::Left)])
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

        print_text(format!("Qualification of {} finished successfully!", ctx.to_version));

        Ok(())
    }
}

enum Steps {
    EnsureBlessedVersions(EnsureBlessedRevisions),
    UpgradeDeploymentCanisters(UpgradeDeploymentCanisters),
    UpgradeSubnets(UpgradeSubnets),
    RetireBlessedVersions(RetireBlessedVersions),
}

pub trait Step {
    fn help(&self) -> String;

    fn name(&self) -> String;

    async fn execute(&self, ctx: &QualificationContext) -> anyhow::Result<()>;

    async fn print_status(&self, ctx: &QualificationContext) -> anyhow::Result<()>;
}

impl Step for Steps {
    fn help(&self) -> String {
        match &self {
            Steps::EnsureBlessedVersions(c) => c.help(),
            Steps::UpgradeDeploymentCanisters(c) => c.help(),
            Steps::UpgradeSubnets(c) => c.help(),
            Steps::RetireBlessedVersions(c) => c.help(),
        }
    }

    fn name(&self) -> String {
        match &self {
            Steps::EnsureBlessedVersions(c) => c.name(),
            Steps::UpgradeDeploymentCanisters(c) => c.name(),
            Steps::UpgradeSubnets(c) => c.name(),
            Steps::RetireBlessedVersions(c) => c.name(),
        }
    }

    async fn execute(&self, ctx: &QualificationContext) -> anyhow::Result<()> {
        match &self {
            Steps::EnsureBlessedVersions(c) => c.execute(ctx).await,
            Steps::UpgradeDeploymentCanisters(c) => c.execute(ctx).await,
            Steps::UpgradeSubnets(c) => c.execute(ctx).await,
            Steps::RetireBlessedVersions(c) => c.execute(ctx).await,
        }
    }

    async fn print_status(&self, ctx: &QualificationContext) -> anyhow::Result<()> {
        match &self {
            Steps::EnsureBlessedVersions(c) => c.print_status(ctx).await,
            Steps::UpgradeDeploymentCanisters(c) => c.print_status(ctx).await,
            Steps::UpgradeSubnets(c) => c.print_status(ctx).await,
            Steps::RetireBlessedVersions(c) => c.print_status(ctx).await,
        }
    }
}
const MAX_RETIRES: usize = 10;
pub async fn ic_admin_with_retry(ic_admin: Arc<IcAdminWrapper>, cmd: ProposeCommand, opts: ProposeOptions) -> anyhow::Result<()> {
    let mut retries = 0;
    backoff::future::retry(backoff::ExponentialBackoff::default(), || {
        let current_opts = opts.clone();
        let current_cmd = cmd.clone();
        let current_admin = ic_admin.clone();
        async move {
            match current_admin.propose_run(current_cmd, current_opts).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    print_text(format!("Retry count {}, failed to place proposal: {}", retries, e));

                    retries += 1;
                    if retries >= MAX_RETIRES {
                        return Err(backoff::Error::Permanent(anyhow::anyhow!("Max retries exceeded")));
                    }
                    Err(backoff::Error::Transient {
                        err: anyhow::anyhow!("Max retries exceeded"),
                        retry_after: None,
                    })
                }
            }
        }
    })
    .await
}

pub async fn print_subnet_versions(registry: Rc<LazyRegistry>) -> anyhow::Result<()> {
    let subnets = registry.subnets().await?;

    let subnets = subnets.values();
    let unassigned = registry.unassigned_nodes_replica_version()?;
    let table = Table::new()
        .with_columns(&[
            ("Subnet type", ColumnAlignment::Left),
            ("Subnet Id", ColumnAlignment::Middle),
            ("Version", ColumnAlignment::Middle),
        ])
        .with_rows(
            subnets
                .map(|s| {
                    vec![
                        match s.subnet_type {
                            SubnetType::Application => "application".to_string(),
                            SubnetType::System => "system".to_string(),
                            SubnetType::VerifiedApplication => "verified-app".to_string(),
                        },
                        s.principal.to_string(),
                        s.replica_version.clone(),
                    ]
                })
                .chain(vec![vec!["unassigned".to_string(), "unassigned".to_string(), unassigned.to_string()]])
                .collect_vec(),
        )
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
