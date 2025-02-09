use clap::Args;
use ic_types::PrincipalId;

use crate::{
    commands::{AuthRequirement, ExecutableCommand},
    forum::{ic_admin::forum_enabled_proposer, ForumPostKind},
    ic_admin::{self},
};

use crate::forum::ForumParameters;

#[derive(Args, Debug)]
pub struct Add {
    /// Node IDs to turn into API BNs
    #[clap(long, num_args(1..), required = true)]
    pub nodes: Vec<PrincipalId>,

    /// guestOS version
    #[clap(long, required = true)]
    pub version: String,

    /// Motivation for creating the subnet
    #[clap(short, long, aliases = ["summary"], required = true)]
    pub motivation: Option<String>,

    #[clap(flatten)]
    pub forum_parameters: ForumParameters,
}

impl ExecutableCommand for Add {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        forum_enabled_proposer(&self.forum_parameters, &ctx, ctx.ic_admin().await?)
            .propose_with_possible_confirmation(
                ic_admin::ProposeCommand::AddApiBoundaryNodes {
                    nodes: self.nodes.to_vec(),
                    version: self.version.clone(),
                },
                ic_admin::ProposeOptions {
                    title: Some(format!("Add {} API boundary node(s)", self.nodes.len())),
                    summary: Some(format!("Add {} API boundary node(s)", self.nodes.len())),
                    motivation: self.motivation.clone(),
                    forum_post_link: None,
                },
                ForumPostKind::Generic,
            )
            .await
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
