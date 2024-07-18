use clap::Args;
use ic_management_types::Network;
use registry_canister::mutations::do_delete_subnet::NNS_SUBNET_ID;
use serde_json::Value;

use crate::{
    commands::{ExecutableCommand, IcAdminRequirement},
    qualification::{QualificationContext, QualificationExecutor},
};

#[derive(Args, Debug)]
pub struct Execute {
    /// Version which is to be qualified
    #[clap(long, short)]
    version: String,

    /// Starting version for the network.
    ///
    /// If left empty, the tool will use the current NNS version
    #[clap(long, short)]
    from_version: Option<String>,
}

impl ExecutableCommand for Execute {
    fn require_ic_admin(&self) -> crate::commands::IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    fn validate(&self, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        if ctx.network().eq(&Network::mainnet_unchecked().unwrap()) {
            anyhow::bail!("Qualification is not allowed on mainnet.")
        }

        let from_version = match &self.from_version {
            Some(v) => v.to_string(),
            None => {
                let anonymous_admin_wrapper_for_mainnet = ctx.readonly_ic_admin_for_other_network(Network::mainnet_unchecked().unwrap());

                let output = anonymous_admin_wrapper_for_mainnet
                    .run_passthrough_get(&["subnet".to_string(), NNS_SUBNET_ID.to_string()], true)
                    .await?;

                let output = serde_json::from_str::<Value>(&output)?;
                output["records"][0]["value"]["replica_version_id"]
                    .as_str()
                    .ok_or(anyhow::anyhow!("Failed to get replica version id for nns"))?
                    .to_string()
            }
        };

        let context = QualificationContext::new(ctx)
            .with_from_version(from_version)
            .with_to_version(self.version.clone());
        let qualification_executor = QualificationExecutor::new(&context);
        qualification_executor.execute(context).await
    }
}
