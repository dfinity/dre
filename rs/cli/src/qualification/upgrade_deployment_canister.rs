use super::Step;

pub struct UpgradeDeploymentCanisters {}

impl Step for UpgradeDeploymentCanisters {
    fn help(&self) -> String {
        "This step ensures that deployment canisters match the version of nns deployment canister".to_string()
    }

    fn name(&self) -> String {
        "update_deployment_canisters".to_string()
    }

    async fn execute(&self, _ctx: &super::QualificationContext) -> anyhow::Result<()> {
        Ok(())
    }

    async fn print_status(&self, _ctx: &super::QualificationContext) -> anyhow::Result<()> {
        Ok(())
    }
}
