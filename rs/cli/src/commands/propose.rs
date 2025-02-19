use clap::{error::ErrorKind, Args};

use crate::{
    forum::{ForumPostKind, SubmissionParameters, Submitter},
    ic_admin::{IcAdminProposal, IcAdminProposalCommand},
};

use super::{AuthRequirement, ExecutableCommand};

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
            .propose(ctx.ic_admin_executor().await?.execution(cmd), ForumPostKind::Generic)
            .await
    }

    fn validate(&self, args: &crate::commands::Args, cmd: &mut clap::Command) {
        let thisargs = match &args.subcommands {
            super::Subcommands::Propose(p) => p.args.clone(),
            _ => Vec::new(),
        };
        if thisargs.iter().any(|arg| {
            ["--forum", "--proposal-url", "--forum-post-link"]
                .iter()
                .any(|other| other == arg || arg.starts_with((other.to_string() + "=").as_str()))
        }) {
            cmd.error(
                ErrorKind::ValueValidation,
                "Option --forum-post-link (or any of its variants) must appear prior to the propose verb or immediately after.",
            )
            .exit()
        }
    }
}
