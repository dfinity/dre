use clap::Args;
use ic_types::PrincipalId;

use crate::{
    commands::{AuthRequirement, ExecutableCommand},
    forum::{ic_admin::forum_enabled_proposer, ForumParameters, ForumPostKind},
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
        forum_enabled_proposer(&self.forum_parameters, &ctx, ctx.ic_admin().await?)
            .propose_run(
                ic_admin::ProposeCommand::RemoveApiBoundaryNodes { nodes: self.nodes.to_vec() },
                ic_admin::ProposeOptions {
                    title: Some(format!("Remove {} API boundary node(s)", self.nodes.len())),
                    summary: Some(format!("Remove {} API boundary node(s)", self.nodes.len())),
                    motivation: self.motivation.clone(),
                    forum_post_link: None,
                },
                ForumPostKind::Generic,
            )
            .await
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
