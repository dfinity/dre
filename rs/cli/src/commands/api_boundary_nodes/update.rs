use clap::Args;
use ic_types::PrincipalId;

use crate::{
    auth::AuthRequirement,
    ctx::exe::ExecutableCommand,
    forum::ForumPostKind,
    ic_admin::{self},
    submitter::{SubmissionParameters, Submitter},
};

#[derive(Args, Debug)]
pub struct Update {
    /// Node IDs where to rollout the version
    #[clap(long, num_args(1..), required = true)]
    pub nodes: Vec<PrincipalId>,

    #[clap(long, required = true)]
    pub version: String,

    /// Motivation for creating the subnet
    #[clap(short, long, aliases = ["summary"], required = true)]
    pub motivation: Option<String>,

    #[clap(flatten)]
    pub submission_parameters: SubmissionParameters,
}

impl ExecutableCommand for Update {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Submitter::from(&self.submission_parameters)
            .propose(
                ctx.ic_admin_executor().await?.execution(ic_admin::IcAdminProposal::new(
                    ic_admin::IcAdminProposalCommand::DeployGuestosToSomeApiBoundaryNodes {
                        nodes: self.nodes.to_vec(),
                        version: self.version.to_string(),
                    },
                    ic_admin::IcAdminProposalOptions {
                        title: Some(format!("Update {} API boundary node(s) to {}", self.nodes.len(), &self.version)),
                        summary: Some(format!("Update {} API boundary node(s) to {}", self.nodes.len(), &self.version)),
                        motivation: self.motivation.clone(),
                    },
                )),
                ForumPostKind::Generic,
            )
            .await
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
