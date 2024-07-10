use clap::Args;
use ic_types::PrincipalId;

use crate::commands::{ExecutableCommand, IcAdminRequirement};

#[derive(Args, Debug)]
pub struct Deploy {
    /// Version to propose for the subnet
    #[clap(long, short)]
    pub version: String,

    /// The ID of the subnet.
    #[clap(long, short)]
    pub id: PrincipalId,
}

impl ExecutableCommand for Deploy {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner().await;
        runner.deploy(&self.id, &self.version).await
    }

    fn validate(&self, cmd: &mut clap::Command) {}
}
