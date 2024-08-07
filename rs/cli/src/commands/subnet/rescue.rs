use clap::Args;
use ic_types::PrincipalId;

use crate::commands::{ExecutableCommand, IcAdminRequirement};

#[derive(Args, Debug)]
pub struct Rescue {
    /// Node features or Node IDs to exclude from the replacement
    #[clap(long, num_args(1..))]
    pub keep_nodes: Option<Vec<String>>,

    /// The ID of the subnet.
    #[clap(long, short)]
    pub id: PrincipalId,
}

impl ExecutableCommand for Rescue {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner().await;

        runner.subnet_rescue(&self.id, self.keep_nodes.clone()).await
    }

    fn validate(&self, _cmd: &mut clap::Command) {}
}
