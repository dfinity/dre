use clap::Args;
use ic_types::PrincipalId;

use crate::{
    commands::{AuthRequirement, ExecutableCommand},
    forum::ForumPostKind,
    ic_admin::{self},
    submitter::{SubmissionParameters, Submitter},
};

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
    pub submission_parameters: SubmissionParameters,
}

impl ExecutableCommand for Add {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Submitter::from(&self.submission_parameters)
            .propose(
                ctx.ic_admin_executor().await?.execution(ic_admin::IcAdminProposal::new(
                    ic_admin::IcAdminProposalCommand::AddApiBoundaryNodes {
                        nodes: self.nodes.to_vec(),
                        version: self.version.clone(),
                    },
                    ic_admin::IcAdminProposalOptions {
                        title: Some(format!("Add {} API boundary node(s)", self.nodes.len())),
                        summary: Some(format!("Add {} API boundary node(s)", self.nodes.len())),
                        motivation: self.motivation.clone(),
                    },
                )),
                ForumPostKind::Generic,
            )
            .await
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
