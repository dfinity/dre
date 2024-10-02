use clap::Args;
use ic_types::PrincipalId;

use crate::commands::{AuthRequirement, ExecutableCommand};

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
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner().await?;

        if let Some(runner_proposal) = runner.subnet_rescue(&self.id, self.keep_nodes.clone(), ctx.forum_post_link()).await? {
            let ic_admin = ctx.ic_admin().await?;
            ic_admin.propose_run(runner_proposal.cmd, runner_proposal.opts).await?;
        }

        Ok(())
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
