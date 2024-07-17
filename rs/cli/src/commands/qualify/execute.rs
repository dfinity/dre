use clap::Args;
use ic_management_types::Network;

use crate::{
    commands::{ExecutableCommand, IcAdminRequirement},
    qualification::{QualificationContext, QualificationExecutor},
};

#[derive(Args, Debug)]
pub struct Execute {}

impl ExecutableCommand for Execute {
    fn require_ic_admin(&self) -> crate::commands::IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    fn validate(&self, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        if ctx.network().eq(&Network::mainnet_unchecked().unwrap()) {
            anyhow::bail!("Qualification is forbidden on mainnet.")
        }

        let qualification_executor = QualificationExecutor::with_steps();
        let context = QualificationContext::new(ctx);
        qualification_executor.execute(context).await
    }
}
