use std::{
    io::{Read, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    rc::Rc,
    str::FromStr,
    time::Duration,
};

use backon::{ExponentialBuilder, Retryable};
use chrono::Utc;
use comfy_table::CellAlignment;
use comfy_table_util::Table;
use ensure_blessed_versions::EnsureBlessedRevisions;
use flate2::bufread::GzDecoder;
use ic_management_backend::lazy_registry::LazyRegistry;
use ic_registry_subnet_type::SubnetType;
use itertools::Itertools;
use reqwest::ClientBuilder;
use retire_blessed_versions::RetireBlessedVersions;
use run_workload_test::Workload;
use run_xnet_test::RunXnetTest;
use upgrade_deployment_canister::UpgradeDeploymentCanisters;
use upgrade_subnets::{Action, UpgradeSubnets};

use crate::ctx::DreContext;

mod comfy_table_util;
mod ensure_blessed_versions;
mod retire_blessed_versions;
mod run_workload_test;
mod run_xnet_test;
mod upgrade_deployment_canister;
mod upgrade_subnets;

pub struct QualificationExecutor {
    steps: Vec<OrderedStep>,
    dre_ctx: DreContext,
    from_version: String,
    to_version: String,
}

struct OrderedStep {
    index: usize,
    should_skip: bool,
    step: Steps,
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
            from_version: "<from-version>".to_string(),
            to_version: "<to-version>".to_string(),
            step_range: "".to_string(),
            deployment_name: "<network-name>".to_string(),
            prometheus_endpoint: "".to_string(),
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

    pub fn build(self) -> QualificationExecutor {
        QualificationExecutor::_new(self)
    }
}

impl QualificationExecutor {
    fn _new(ctx: QualificationExecutorBuilder) -> Self {
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
        Self {
            steps: steps
                .into_iter()
                .enumerate()
                .map(|(i, s)| OrderedStep {
                    index: i,
                    should_skip: !(start_index <= i && i <= end_index),
                    step: s,
                })
                .collect_vec(),
            dre_ctx: ctx.dre_ctx,
            from_version: ctx.from_version,
            to_version: ctx.to_version,
        }
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

        println!("{}", table)
    }

    pub async fn execute(&self) -> anyhow::Result<()> {
        print_text("This qualification run will execute the following steps:".to_string());
        self.list();

        print_text(format!("Running qualification from version {} to {}", self.from_version, self.to_version));
        print_text(format!("Starting execution of {} steps:", self.steps.len()));
        for ordered_step in &self.steps {
            if ordered_step.should_skip {
                print_text(format!(
                    "Skipping step {} due to skip-range: `{}`",
                    ordered_step.index,
                    ordered_step.step.name()
                ));
                continue;
            }
            print_text(format!("Executing step {}: `{}`", ordered_step.index, ordered_step.step.name()));

            let step_future = || async { ordered_step.step.execute(&self.dre_ctx).await };
            step_future.retry(&ExponentialBuilder::default()).await?;

            print_text(format!("Executed step {}: `{}`", ordered_step.index, ordered_step.step.name()));

            let registry = self.dre_ctx.registry().await;
            print_text(format!("Syncing with registry after step {}", ordered_step.index));
            let sync_registry = || async { registry.sync_with_nns().await };
            // If the system subnet downgraded it could be some time until it boots up
            sync_registry
                .retry(
                    &ExponentialBuilder::default()
                        .with_max_times(10)
                        .with_max_delay(Duration::from_secs(5 * 60)),
                )
                .await?;
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
    RunWorkloadTest(Workload),
    RunXnetTest(RunXnetTest),
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
            Steps::RunWorkloadTest(c) => c.help(),
            Steps::RunXnetTest(c) => c.help(),
        }
    }

    fn name(&self) -> String {
        match &self {
            Steps::EnsureBlessedVersions(c) => c.name(),
            Steps::UpgradeDeploymentCanisters(c) => c.name(),
            Steps::UpgradeSubnets(c) => c.name(),
            Steps::RetireBlessedVersions(c) => c.name(),
            Steps::RunWorkloadTest(c) => c.name(),
            Steps::RunXnetTest(c) => c.name(),
        }
    }

    async fn execute(&self, ctx: &DreContext) -> anyhow::Result<()> {
        match &self {
            Steps::EnsureBlessedVersions(c) => c.execute(ctx).await,
            Steps::UpgradeDeploymentCanisters(c) => c.execute(ctx).await,
            Steps::UpgradeSubnets(c) => c.execute(ctx).await,
            Steps::RetireBlessedVersions(c) => c.execute(ctx).await,
            Steps::RunWorkloadTest(c) => c.execute(ctx).await,
            Steps::RunXnetTest(c) => c.execute(ctx).await,
        }
    }
}

const REQWEST_TIMEOUT: Duration = Duration::from_secs(30);
const IC_EXECUTABLES_DIR: &str = "ic-executables";
pub async fn download_canister(canister: &str, version: &str) -> anyhow::Result<PathBuf> {
    let client = ClientBuilder::new().timeout(REQWEST_TIMEOUT).build()?;

    let cache = dirs::cache_dir().ok_or(anyhow::anyhow!("Can't cache dir"))?.join(IC_EXECUTABLES_DIR);
    if !cache.exists() {
        std::fs::create_dir_all(&cache)?;
    }

    let artifact_path = cache.join(format!("{}/{}.{}", canister, canister, version));
    let artifact_dir = artifact_path.parent().unwrap();
    if !artifact_dir.exists() {
        std::fs::create_dir(artifact_dir)?;
    }

    let canister_path = PathBuf::from_str(&format!("{}.wasm", artifact_path.display())).map_err(|e| anyhow::anyhow!(e))?;

    if canister_path.exists() {
        print_text(format!("Canister `{}` data already present", canister));
        return Ok(canister_path);
    }

    let url = format!("https://download.dfinity.systems/ic/{}/canisters/{}.wasm.gz", version, canister);

    print_text(format!("Downloading: {}", url));
    let response = client.get(&url).send().await?.error_for_status()?.bytes().await?;
    let mut d = GzDecoder::new(&response[..]);
    let mut collector: Vec<u8> = vec![];
    let mut file = std::fs::File::create(&canister_path)?;
    d.read_to_end(&mut collector)?;

    file.write_all(&collector)?;
    print_text(format!("Downloaded: {}", &url));
    Ok(canister_path)
}

pub async fn download_executable(executable: &str, version: &str) -> anyhow::Result<PathBuf> {
    let client = ClientBuilder::new().timeout(REQWEST_TIMEOUT).build()?;

    let cache = dirs::cache_dir().ok_or(anyhow::anyhow!("Can't cache dir"))?.join(IC_EXECUTABLES_DIR);
    if !cache.exists() {
        std::fs::create_dir_all(&cache)?;
    }

    let exe_path = cache.join(format!("{}/{}.{}", executable, executable, version));
    let artifact_dir = exe_path.parent().unwrap();
    if !artifact_dir.exists() {
        std::fs::create_dir(artifact_dir)?;
    }

    if exe_path.exists() && exe_path.is_file() {
        let permissions = exe_path.metadata()?.permissions();
        let is_executable = permissions.mode() & 0o111 != 0;
        if is_executable {
            print_text(format!("Executable `{}` already present and executable", executable));
            return Ok(exe_path);
        }
    }

    let url = format!(
        "https://download.dfinity.systems/ic/{}/binaries/x86_64-{}/{}.gz",
        version,
        match std::env::consts::OS {
            "linux" => "linux",
            "macos" => "darwin",
            s => return Err(anyhow::anyhow!("Unsupported os: {}", s)),
        },
        executable
    );

    print_text(format!("Downloading: {}", url));
    let response = client.get(&url).send().await?.error_for_status()?.bytes().await?;
    let mut d = GzDecoder::new(&response[..]);
    let mut collector: Vec<u8> = vec![];
    let mut file = std::fs::File::create(&exe_path)?;
    d.read_to_end(&mut collector)?;

    file.write_all(&collector)?;
    print_text(format!("Downloaded: {}", &url));

    file.set_permissions(PermissionsExt::from_mode(0o774))?;
    print_text(format!("Created executable: {}", exe_path.display()));
    Ok(exe_path)
}

pub async fn print_subnet_versions(registry: Rc<LazyRegistry>) -> anyhow::Result<()> {
    let subnets = registry.subnets().await?;

    let subnets = subnets.values();
    let unassigned = registry.unassigned_nodes_replica_version()?;
    let table = Table::new()
        .with_columns(&[
            ("Subnet type", CellAlignment::Left),
            ("Subnet Id", CellAlignment::Center),
            ("Version", CellAlignment::Center),
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

pub fn print_table(table: comfy_table::Table) {
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
