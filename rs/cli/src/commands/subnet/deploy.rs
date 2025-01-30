use clap::Args;

use ic_types::PrincipalId;

use crate::{
    auth::get_automation_neuron_default_path,
    commands::{AuthRequirement, ExecutableCommand},
    forum::{ic_admin::forum_enabled_proposer, ForumParameters, ForumPostKind},
};

#[derive(Args, Debug)]
#[clap(visible_aliases = &["upgrade", "update"])]
pub struct Deploy {
    /// Version to propose for the subnet
    #[clap(long, short)]
    pub version: String,

    /// The ID of the subnet.
    #[clap(long, short)]
    pub id: PrincipalId,

    #[clap(flatten)]
    pub forum_parameters: ForumParameters,
}

impl ExecutableCommand for Deploy {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner_proposal = ctx.runner().await?.deploy(&self.id, &self.version, None).await?;
        forum_enabled_proposer(&self.forum_parameters, &ctx, ctx.ic_admin().await?)
            .propose_run(runner_proposal.cmd, runner_proposal.opts, ForumPostKind::Generic)
            .await
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
