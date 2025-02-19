use clap::Args;

use ic_types::PrincipalId;

use crate::{
    auth::get_automation_neuron_default_path,
    commands::{AuthRequirement, ExecutableCommand},
    forum::{ForumPostKind, SubmissionParameters, Submitter},
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
    pub submission_parameters: SubmissionParameters,
}

impl ExecutableCommand for Deploy {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner_proposal = ctx.runner().await?.deploy(&self.id, &self.version).await?;
        Submitter::from(&self.submission_parameters)
            .propose(ctx.ic_admin_executor().await?.execution(runner_proposal), ForumPostKind::Generic)
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
