use clap::{error::ErrorKind, Args};

use crate::{
    ctx::HowToProceed,
    forum::{ForumParameters, ForumPostKind, Submitter},
    ic_admin::{IcAdminProposal, IcAdminProposalCommand},
};

use super::{AuthRequirement, ExecutableCommand};

#[derive(Args, Debug)]
pub struct Propose {
    #[clap(flatten)]
    pub forum_parameters: ForumParameters,

    /// Arbitrary ic-admin args
    #[clap(allow_hyphen_values = true)]
    pub args: Vec<String>,
}

fn disambiguate_mode_from_ctx_and_postargs(mode: HowToProceed, postargs: &[String]) -> anyhow::Result<HowToProceed> {
    let dry_run_in_args = postargs.iter().any(|e| *e == "--dry-run")
        || postargs.iter().any(|e| *e == "--dryrun")
        || postargs.iter().any(|e| *e == "--simulate")
        || postargs.iter().any(|e| *e == "--no");
    let yes_in_args = postargs.iter().any(|e| *e == "--yes");
    let mode_from_post_args = match (dry_run_in_args, yes_in_args) {
        (true, false) => Ok(HowToProceed::DryRun),
        (false, true) => Ok(HowToProceed::Unconditional),
        (false, false) => Ok(HowToProceed::Confirm),
        (true, true) => Err(anyhow::anyhow!(
            "Conflicting request: cannot specify --yes and --dry-run (or any variant) in the same command."
        )),
    }?;
    if mode == mode_from_post_args {
        Ok(mode)
    } else if HowToProceed::Confirm == mode {
        // No specific mode was specified in the pre-args.  Use whatever post-args we need.
        Ok(mode_from_post_args)
    } else if HowToProceed::Confirm == mode_from_post_args {
        // No specific mode was specified in the pre-args.  Use whatever post-args we need.
        Ok(mode)
    } else {
        Err(anyhow::anyhow!(
            "Conflicting request: cannot specify --yes and --dry-run (or any variant) in the same command."
        ))
    }
}

fn calculate_mode_from_general_args(dry_run: bool, yes: bool) -> anyhow::Result<HowToProceed> {
    match (dry_run, yes) {
        (true, false) => Ok(HowToProceed::DryRun),
        (false, true) => Ok(HowToProceed::Unconditional),
        (false, false) => Ok(HowToProceed::Confirm),
        (true, true) => Err(anyhow::anyhow!(
            "Conflicting request: cannot specify --yes and --dry-run (or any variant) in the same command."
        )),
    }
}

impl ExecutableCommand for Propose {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let mode = disambiguate_mode_from_ctx_and_postargs(ctx.mode.clone(), &self.args)?;

        let args: Vec<String> = self
            .args
            .clone()
            .into_iter()
            .filter(|arg| !["--yes", "--no", "--dry-run", "--dryrun", "--simulate"].iter().any(|other| other == arg))
            .collect();

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

        Submitter::from_executor_and_mode(&self.forum_parameters, mode, ctx.ic_admin_executor().await?.execution(cmd))
            .propose(ForumPostKind::Generic)
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
        let argmode = match calculate_mode_from_general_args(args.dry_run, args.yes) {
            Ok(m) => m,
            Err(e) => cmd.error(ErrorKind::ArgumentConflict, format!("{}", e)).exit(),
        };
        if let Err(e) = disambiguate_mode_from_ctx_and_postargs(argmode, &thisargs) {
            cmd.error(ErrorKind::ArgumentConflict, format!("{}", e)).exit()
        }
    }
}
