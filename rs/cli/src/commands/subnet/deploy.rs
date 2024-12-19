use clap::Args;

use ic_types::PrincipalId;

use crate::{
    auth::get_automation_neuron_default_path,
    commands::{AuthRequirement, ExecutableCommand},
};

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
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner().await?;
        let runner_proposal = runner.deploy(&self.id, &self.version, ctx.forum_post_link()).await?;
        let ic_admin = ctx.ic_admin().await?;
        ic_admin.propose_run(runner_proposal.cmd, runner_proposal.opts).await?;
        Ok(())
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}

    fn neuron_override(&self) -> Option<crate::auth::Neuron> {
        Some(crate::auth::Neuron {
            auth: crate::auth::Auth::Keyfile {
                path: get_automation_neuron_default_path(),
            },
            neuron_id: 80,
            include_proposer: true,
        })
    }
}
