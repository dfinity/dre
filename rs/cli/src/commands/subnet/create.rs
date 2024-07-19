use clap::{error::ErrorKind, Args};
use ic_management_types::requests::SubnetCreateRequest;
use ic_types::PrincipalId;

use crate::commands::{ExecutableCommand, IcAdminRequirement};

#[derive(Args, Debug)]
pub struct Create {
    /// Number of nodes in the subnet
    #[clap(long, default_value_t = 13)]
    pub size: usize,

    /// Minimum nakamoto coefficients desired
    #[clap(long, num_args(1..))]
    pub min_nakamoto_coefficients: Vec<String>,

    /// Features or Node IDs to exclude from the available nodes pool
    #[clap(long, num_args(1..))]
    pub exclude: Vec<String>,

    /// Features or node IDs to only choose from
    #[clap(long, num_args(1..))]
    pub only: Vec<String>,

    #[clap(long, num_args(1..), help = r#"Force t he inclusion of the provided nodes for replacement,
regardless of the decentralization score"#)]
    pub include: Vec<PrincipalId>,

    /// Motivation for replacing custom nodes
    #[clap(long, short, aliases = [ "summary" ])]
    pub motivation: Option<String>,

    /// Replica version to use for the new subnet
    #[clap(long)]
    pub replica_version: Option<String>,

    /// Arbitrary other ic-args
    #[clap(allow_hyphen_values = true)]
    other_args: Vec<String>,

    /// Provide the list of all arguments that ic-admin accepts for subnet creation
    #[clap(long)]
    pub help_other_args: bool,
}

impl ExecutableCommand for Create {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::default()
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner().await;
        let motivation = match &self.motivation {
            Some(m) => m,
            None if self.help_other_args => &"help for options".to_string(),
            None => unreachable!("Should be caught by validate()"),
        };

        runner
            .subnet_create(
                SubnetCreateRequest {
                    size: self.size,
                    min_nakamoto_coefficients: Self::parse_min_nakamoto_coefficients(&self.min_nakamoto_coefficients),
                    exclude: self.exclude.clone().into(),
                    only: self.only.clone().into(),
                    include: self.include.clone().into(),
                },
                motivation.to_string(),
                self.replica_version.clone(),
                self.other_args.to_owned(),
                self.help_other_args,
            )
            .await
    }

    fn validate(&self, cmd: &mut clap::Command) {
        if self.motivation.is_none() && !self.help_other_args {
            cmd.error(
                ErrorKind::MissingRequiredArgument,
                "Motivation is required if `--help-other-args` is not provided",
            )
            .exit()
        }

        Self::validate_min_nakamoto_coefficients(cmd, &self.min_nakamoto_coefficients);
    }
}
