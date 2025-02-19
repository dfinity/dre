use clap::Args;
use ic_types::PrincipalId;

use crate::{
    commands::{AuthRequirement, ExecutableCommand},
    forum::{ForumParameters, ForumPostKind, Submitter},
    ic_admin::{self},
};

#[derive(Args, Debug)]
pub struct Remove {
    /// Node IDs of API BNs that should be turned into unassigned nodes again
    #[clap(long, num_args(1..), required = true)]
    pub nodes: Vec<PrincipalId>,

    /// Motivation for removing the API BNs
    #[clap(short, long, aliases = ["summary"], required = true)]
    pub motivation: Option<String>,

    #[clap(flatten)]
    pub forum_parameters: ForumParameters,
}

impl ExecutableCommand for Remove {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Submitter::from_executor_and_mode(
            &self.forum_parameters,
            ctx.mode.clone(),
            ctx.ic_admin_executor().await?.execution(ic_admin::IcAdminProposal::new(
                ic_admin::IcAdminProposalCommand::RemoveApiBoundaryNodes { nodes: self.nodes.to_vec() },
                ic_admin::IcAdminProposalOptions {
                    title: Some(format!("Remove {} API boundary node(s)", self.nodes.len())),
                    summary: Some(format!("Remove {} API boundary node(s)", self.nodes.len())),
                    motivation: self.motivation.clone(),
                },
            )),
        )
        .propose(ForumPostKind::Generic)
        .await
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
