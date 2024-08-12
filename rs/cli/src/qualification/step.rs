use super::{
    ensure_blessed_versions::EnsureBlessedRevisions, retire_blessed_versions::RetireBlessedVersions, run_workload_test::Workload,
    run_xnet_test::RunXnetTest, upgrade_deployment_canister::UpgradeDeploymentCanisters, upgrade_subnets::UpgradeSubnets, util::StepCtx,
};

pub struct OrderedStep {
    pub index: usize,
    pub should_skip: bool,
    pub step: Steps,
}

pub enum Steps {
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

    async fn execute(&self, ctx: &StepCtx) -> anyhow::Result<()>;
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

    async fn execute(&self, ctx: &StepCtx) -> anyhow::Result<()> {
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
