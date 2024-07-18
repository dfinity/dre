use super::Step;

#[derive(Default)]
pub struct UpgradeDeploymentCanisters {}

impl Step for UpgradeDeploymentCanisters {
    fn help(&self) -> &'static str {
        "This step ensures that deployment canisters match the version of nns deployment canister"
    }

    fn name(&self) -> &'static str {
        "1c_update_deployment_canisters"
    }

    async fn execute(&self, _ctx: &super::QualificationContext) -> anyhow::Result<()> {
        Ok(())
    }

    async fn print_status(&self, _ctx: &super::QualificationContext) -> anyhow::Result<()> {
        Ok(())
    }
}
