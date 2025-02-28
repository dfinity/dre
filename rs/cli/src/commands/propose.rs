use clap::{error::ErrorKind, Args};

use crate::auth::AuthRequirement;
use crate::exe::args::GlobalArgs;
use crate::exe::ExecutableCommand;
use crate::{
    forum::ForumPostKind,
    ic_admin::{IcAdminProposal, IcAdminProposalCommand},
    submitter::{SubmissionParameters, Submitter},
};

#[derive(Args, Debug)]
pub struct Propose {
    #[clap(flatten)]
    pub submission_parameters: SubmissionParameters,

    /// Arbitrary ic-admin args
    #[clap(allow_hyphen_values = true)]
    pub args: Vec<String>,
}

impl ExecutableCommand for Propose {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let args: Vec<String> = self.args.clone().into_iter().collect();

        // ic-admin expects --summary and not --motivation
        // make sure the expected argument is provided
        let args = if !args.contains(&String::from("--summary")) && args.contains(&String::from("--motivation")) {
            args.iter()
                .map(|arg| if arg == "--motivation" { "--summary".to_string() } else { arg.clone() })
                .collect::<Vec<_>>()
        } else {
            args.to_vec()
        };

        if args.is_empty() {
            return ctx.help_propose(None).await;
        }

        let cmd = IcAdminProposal::new(IcAdminProposalCommand::Raw(args.clone()), Default::default());

        Submitter::from(&self.submission_parameters)
            .propose_and_print(ctx.ic_admin_executor().await?.execution(cmd), ForumPostKind::Generic)
            .await
    }

    fn validate(&self, _args: &GlobalArgs, cmd: &mut clap::Command) {
        if self.args.iter().any(|arg| {
            ["--forum", "--proposal-url", "--forum-post-link", "--yes", "--dry-run", "--no"]
                .iter()
                .any(|other| other == arg || arg.starts_with((other.to_string() + "=").as_str()))
        }) {
            cmd.error(
                ErrorKind::ValueValidation,
                "Some of the options you used must appear immediately after the `propose` verb.",
            )
            .exit()
        }
    }
}
