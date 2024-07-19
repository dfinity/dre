use std::{rc::Rc, sync::Arc};

use chrono::Utc;
use ensure_blessed_versions::EnsureBlessedRevisions;
use ic_management_backend::lazy_registry::LazyRegistry;
use ic_registry_subnet_type::SubnetType;
use itertools::Itertools;
use retire_blessed_versions::RetireBlessedVersions;
use run_xnet_test::XNetTest;
use tabular_util::{ColumnAlignment, Table};
use upgrade_deployment_canister::UpgradeDeploymentCanisters;
use upgrade_subnets::{Action, UpgradeSubnets};

use crate::{
    ctx::DreContext,
    ic_admin::{IcAdminWrapper, ProposeCommand, ProposeOptions},
};

mod ensure_blessed_versions;
mod retire_blessed_versions;
mod run_xnet_test;
mod tabular_util;
mod upgrade_deployment_canister;
mod upgrade_subnets;

pub struct QualificationExecutor {
    steps: Vec<(usize, bool, Steps)>,
    dre_ctx: DreContext,
    from_version: String,
    to_version: String,
}

pub struct QualificationExecutorBuilder {
    dre_ctx: DreContext,
    from_version: String,
    to_version: String,
    step_range: String,
    deployment_name: String,
    prometheus_endpoint: String,
}

impl QualificationExecutorBuilder {
    pub fn new(dre_ctx: DreContext) -> Self {
        Self {
            dre_ctx,
            from_version: "".to_string(),
            to_version: "".to_string(),
            step_range: "".to_string(),
            deployment_name: "".to_string(),
            prometheus_endpoint: "".to_string(),
        }
    }

    pub fn from_version(self, from_version: String) -> Self {
        Self { from_version, ..self }
    }

    pub fn to_version(self, to_version: String) -> Self {
        Self { to_version, ..self }
    }

    pub fn with_step_range(self, step_range: String) -> Self {
        Self { step_range, ..self }
    }

    pub fn with_deployment_namge(self, deployment_name: String) -> Self {
        Self { deployment_name, ..self }
    }

    pub fn with_prometheus_endpoint(self, prometheus_endpoint: String) -> Self {
        Self { prometheus_endpoint, ..self }
    }

    pub fn build(self) -> QualificationExecutor {
        QualificationExecutor::_new(self)
    }
}

impl QualificationExecutor {
    fn _new(ctx: QualificationExecutorBuilder) -> Self {
        let steps = vec![
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
            // Run xnet tests
            Steps::RunXnetTest(XNetTest {
                version: ctx.to_version.clone(),
                deployment_name: ctx.deployment_name.clone(),
                prometheus_endpoint: ctx.prometheus_endpoint.clone(),
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
        ];

        let (start_index, end_index) = if ctx.step_range.contains("..") {
            let split = ctx.step_range.split("..").map(|f| f.to_string()).collect_vec();
            let first = split.get(0).map(|s| s.parse::<usize>().unwrap_or(0)).unwrap_or(0);
            let last = split
                .get(1)
                .map(|s| s.parse::<usize>().unwrap_or(steps.len() - 1))
                .unwrap_or(steps.len() - 1);
            (first, last)
        } else {
            match ctx.step_range.parse::<usize>() {
                Ok(v) => (v, v),
                Err(_) => (0, steps.len() - 1),
            }
        };

        let (start_index, end_index) = match start_index.cmp(&end_index) {
            std::cmp::Ordering::Less | std::cmp::Ordering::Equal => (start_index, end_index),
            std::cmp::Ordering::Greater => (0, steps.len() - 1),
        };

        let end_index = if end_index > steps.len() - 1 { steps.len() - 1 } else { end_index };
        Self {
            steps: steps
                .into_iter()
                .enumerate()
                .map(|(i, s)| (i, !(start_index <= i && i <= end_index), s))
                .collect_vec(),
            dre_ctx: ctx.dre_ctx,
            from_version: ctx.from_version,
            to_version: ctx.to_version,
        }
    }

    pub fn list(&self) {
        let table = Table::new()
            .with_columns(&[
                ("Index", ColumnAlignment::Middle),
                ("Will run", ColumnAlignment::Middle),
                ("Name", ColumnAlignment::Left),
                ("Help", ColumnAlignment::Left),
            ])
            .with_rows(
                self.steps
                    .iter()
                    .map(|(i, sk, s)| vec![(i).to_string(), (!sk).to_string(), s.name().to_string(), s.help().to_string()])
                    .collect_vec(),
            )
            .to_table();

        println!("{}", table)
    }

    pub async fn execute(&self) -> anyhow::Result<()> {
        print_text("This qualification run will execute the following steps:".to_string());
        self.list();

        print_text(format!("Running qualification from version {} to {}", self.from_version, self.to_version));
        print_text(format!("Starting execution of {} steps:", self.steps.len()));
        for (i, sk, step) in self.steps.iter() {
            if *sk {
                print_text(format!("Skipping step {} due to skip-range: `{}`", i, step.name()));
                continue;
            }
            print_text(format!("Executing step {}: `{}`", i, step.name()));

            step.execute(&self.dre_ctx).await?;

            print_text(format!("Executed step {}: `{}`", i, step.name()));

            let registry = self.dre_ctx.registry().await;
            print_text(format!("Syncing with registry after step {}", i));
            registry.sync_with_nns().await?;
        }

        print_text(format!("Qualification of {} finished successfully!", self.to_version));

        Ok(())
    }
}

enum Steps {
    EnsureBlessedVersions(EnsureBlessedRevisions),
    UpgradeDeploymentCanisters(UpgradeDeploymentCanisters),
    UpgradeSubnets(UpgradeSubnets),
    RetireBlessedVersions(RetireBlessedVersions),
    RunXnetTest(XNetTest),
}

pub trait Step {
    fn help(&self) -> String;

    fn name(&self) -> String;

    async fn execute(&self, ctx: &DreContext) -> anyhow::Result<()>;
}

impl Step for Steps {
    fn help(&self) -> String {
        match &self {
            Steps::EnsureBlessedVersions(c) => c.help(),
            Steps::UpgradeDeploymentCanisters(c) => c.help(),
            Steps::UpgradeSubnets(c) => c.help(),
            Steps::RetireBlessedVersions(c) => c.help(),
            Steps::RunXnetTest(c) => c.help(),
        }
    }

    fn name(&self) -> String {
        match &self {
            Steps::EnsureBlessedVersions(c) => c.name(),
            Steps::UpgradeDeploymentCanisters(c) => c.name(),
            Steps::UpgradeSubnets(c) => c.name(),
            Steps::RetireBlessedVersions(c) => c.name(),
            Steps::RunXnetTest(c) => c.name(),
        }
    }

    async fn execute(&self, ctx: &DreContext) -> anyhow::Result<()> {
        match &self {
            Steps::EnsureBlessedVersions(c) => c.execute(ctx).await,
            Steps::UpgradeDeploymentCanisters(c) => c.execute(ctx).await,
            Steps::UpgradeSubnets(c) => c.execute(ctx).await,
            Steps::RetireBlessedVersions(c) => c.execute(ctx).await,
            Steps::RunXnetTest(c) => c.execute(ctx).await,
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
