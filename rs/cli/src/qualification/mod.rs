use std::{path::PathBuf, time::Duration};

use backon::{ExponentialBuilder, Retryable};
use comfy_table::CellAlignment;
use comfy_table_util::Table;
use ensure_blessed_versions::EnsureBlessedRevisions;
use ic_registry_subnet_type::SubnetType;
use itertools::Itertools;
use retire_blessed_versions::RetireBlessedVersions;
use run_workload_test::Workload;
use run_xnet_test::RunXnetTest;
use step::{OrderedStep, Step, Steps};
use upgrade_deployment_canister::UpgradeDeploymentCanisters;
use upgrade_subnets::{Action, UpgradeSubnets};
use util::StepCtx;

use crate::ctx::DreContext;

mod comfy_table_util;
mod ensure_blessed_versions;
mod retire_blessed_versions;
mod run_workload_test;
mod run_xnet_test;
mod step;
mod upgrade_deployment_canister;
mod upgrade_subnets;
mod util;

pub struct QualificationExecutor {
    steps: Vec<OrderedStep>,
    from_version: String,
    to_version: String,
    step_ctx: StepCtx,
}

pub struct QualificationExecutorBuilder {
    dre_ctx: DreContext,
    from_version: String,
    to_version: String,
    step_range: String,
    deployment_name: String,
    prometheus_endpoint: String,
    artifacts: Option<PathBuf>,
    grafana_endpoint: Option<String>,
}

impl QualificationExecutorBuilder {
    pub fn new(dre_ctx: DreContext) -> Self {
        Self {
            dre_ctx,
            from_version: "<from-version>".to_string(),
            to_version: "<to-version>".to_string(),
            step_range: "".to_string(),
            deployment_name: "<network-name>".to_string(),
            prometheus_endpoint: "".to_string(),
            artifacts: None,
            grafana_endpoint: None,
        }
    }

    pub fn with_from_version(self, from_version: String) -> Self {
        Self { from_version, ..self }
    }

    pub fn with_to_version(self, to_version: String) -> Self {
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

    pub fn with_artifacts(self, path: PathBuf) -> Self {
        Self {
            artifacts: Some(path),
            ..self
        }
    }

    pub fn with_grafana_endpoint(self, grafana_endpoint: String) -> Self {
        Self {
            grafana_endpoint: Some(grafana_endpoint),
            ..self
        }
    }

    pub fn build(self) -> anyhow::Result<QualificationExecutor> {
        QualificationExecutor::_new(self)
    }
}

impl QualificationExecutor {
    fn _new(ctx: QualificationExecutorBuilder) -> anyhow::Result<Self> {
        let steps = vec![
            // Ensure the beginning version is blessed
            // This step will be skipped for testnet runs, but may be
            // required for staging
            Steps::EnsureBlessedVersions(EnsureBlessedRevisions {
                version: ctx.from_version.clone(),
            }),
            // Ensure app subnets are on beginning version
            // This step will be skipped for testnet runs, but may be
            // required for staging
            Steps::UpgradeSubnets(UpgradeSubnets {
                action: Action::Upgrade,
                subnet_type: Some(SubnetType::Application),
                to_version: ctx.from_version.clone(),
            }),
            // Ensure all system subnets are on beginning version
            // This step will be skipped for testnet runs, but may be
            // required for staging
            Steps::UpgradeSubnets(UpgradeSubnets {
                action: Action::Upgrade,
                subnet_type: Some(SubnetType::System),
                to_version: ctx.from_version.clone(),
            }),
            // Ensure unassigned nodes are on beginning version
            // This step will be skipped for testnet runs, but may be
            // required for staging
            Steps::UpgradeSubnets(UpgradeSubnets {
                action: Action::Upgrade,
                subnet_type: None,
                to_version: ctx.from_version.clone(),
            }),
            // Blessing the version which we are qualifying
            // This step will be run on each network and marks the
            // beginning of a qualification
            Steps::EnsureBlessedVersions(EnsureBlessedRevisions {
                version: ctx.to_version.clone(),
            }),
            // Upgrading deployment canisters
            // TODO: finish this step
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
            // Run workload tests
            // TODO: add artifacts exporting
            Steps::RunWorkloadTest(Workload {
                version: ctx.to_version.clone(),
                deployment_name: ctx.deployment_name.clone(),
                prometheus_endpoint: ctx.prometheus_endpoint.clone(),
            }),
            // Run XNet tests
            // TODO: add artifacts exporting
            Steps::RunXnetTest(RunXnetTest {
                version: ctx.to_version.clone(),
            }),
            // Since the initial testnet is spunup with disk-img
            // retire the initial version.
            // This step may not be required on staging but the version
            // will just be re-elected in the next step
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
            // Run workload tests again
            // TODO: add artifacts exporting
            Steps::RunWorkloadTest(Workload {
                version: ctx.from_version.clone(),
                deployment_name: ctx.deployment_name.clone(),
                prometheus_endpoint: ctx.prometheus_endpoint.clone(),
            }),
            // Run XNet tests again
            // TODO: add artifacts exporting
            Steps::RunXnetTest(RunXnetTest {
                version: ctx.from_version.clone(),
            }),
        ];

        let (start_index, end_index) = if ctx.step_range.contains("..") {
            let split = ctx.step_range.split("..").map(|f| f.to_string()).collect_vec();
            let first = split.first().map(|s| s.parse::<usize>().unwrap_or(0)).unwrap_or(0);
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
        Ok(Self {
            steps: steps
                .into_iter()
                .enumerate()
                .map(|(i, s)| OrderedStep {
                    index: i,
                    should_skip: !(start_index <= i && i <= end_index),
                    step: s,
                })
                .collect_vec(),
            step_ctx: StepCtx::new(ctx.dre_ctx, ctx.artifacts, ctx.to_version.clone())?,
            from_version: ctx.from_version,
            to_version: ctx.to_version,
        })
    }

    pub fn list(&self) {
        let table = Table::new()
            .with_columns(&[
                ("Index", CellAlignment::Center),
                ("Will run", CellAlignment::Center),
                ("Name", CellAlignment::Left),
                ("Help", CellAlignment::Left),
            ])
            .with_rows(
                self.steps
                    .iter()
                    .map(|ordered_step| {
                        vec![
                            ordered_step.index.to_string(),
                            (!ordered_step.should_skip).to_string(),
                            ordered_step.step.name().to_string(),
                            ordered_step.step.help().to_string(),
                        ]
                    })
                    .collect_vec(),
            )
            .to_table();

        self.step_ctx.print_table(table)
    }

    pub async fn execute(&self) -> anyhow::Result<()> {
        self.print_text("This qualification run will execute the following steps:".to_string());
        self.list();

        self.print_text(format!("Running qualification from version {} to {}", self.from_version, self.to_version));
        self.print_text(format!("Starting execution of {} steps:", self.steps.len()));
        for ordered_step in &self.steps {
            if ordered_step.should_skip {
                self.print_text(format!(
                    "Skipping step {} due to skip-range: `{}`",
                    ordered_step.index,
                    ordered_step.step.name()
                ));
                continue;
            }
            self.print_text(format!("Executing step {}: `{}`", ordered_step.index, ordered_step.step.name()));

            let step_future = || async { ordered_step.step.execute(&self.step_ctx).await };
            if let Err(e) = step_future.retry(&ExponentialBuilder::default()).await {
                self.print_text(format!("Failed to execute step {}: {:?}", ordered_step.step.name(), e));
                anyhow::bail!(e)
            }

            self.print_text(format!("Executed step {}: `{}`", ordered_step.index, ordered_step.step.name()));

            let registry = self.step_ctx.dre_ctx().registry().await;
            self.print_text(format!("Syncing with registry after step {}", ordered_step.index));
            let sync_registry = || async { registry.sync_with_nns().await };
            // If the system subnet downgraded it could be some time until it boots up
            if let Err(e) = sync_registry
                .retry(
                    &ExponentialBuilder::default()
                        .with_max_times(10)
                        .with_max_delay(Duration::from_secs(5 * 60)),
                )
                .await
            {
                self.print_text(format!("Failed to sync with registry: {:?}", e));
                anyhow::bail!(e)
            }
        }

        self.print_text(format!("Qualification of {} finished successfully!", self.to_version));

        Ok(())
    }

    fn print_text(&self, message: String) {
        self.step_ctx.print_text(message)
    }
}
