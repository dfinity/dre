use ic_nns_constants::ALL_NNS_CANISTER_IDS;

use super::{step::Step, util::StepCtx};

pub struct UpgradeDeploymentCanisters {}

impl Step for UpgradeDeploymentCanisters {
    fn help(&self) -> String {
        "Ensure that deployment canisters match the version of nns deployment canister".to_string()
    }

    fn name(&self) -> String {
        "update_deployment_canisters".to_string()
    }

    async fn execute(&self, ctx: &StepCtx) -> anyhow::Result<()> {
        for canister_id in ALL_NNS_CANISTER_IDS {
            ctx.print_text(format!("Checking version of canister with id {}", canister_id))
        }

        Ok(())
    }
}
