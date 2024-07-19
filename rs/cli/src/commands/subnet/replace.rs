use clap::{error::ErrorKind, Args};
use ic_types::PrincipalId;

use crate::{
    commands::{ExecutableCommand, IcAdminRequirement},
    subnet_manager::SubnetTarget,
};

#[derive(Args, Debug)]
pub struct Replace {
    /// Set of custom nodes to be replaced
    #[clap(long, short)]
    pub nodes: Vec<PrincipalId>,

    /// Do not replace unhealthy nodes
    #[clap(long)]
    pub no_heal: bool,

    #[clap(
        long,
        short,
        help = r#"Amount of nodes to be replaced by decentralization optimization
algorithm"#
    )]
    pub optimize: Option<usize>,

    /// Motivation for replacing custom nodes
    #[clap(long, short, aliases = [ "summary" ])]
    pub motivation: Option<String>,

    /// Minimum Nakamoto coefficients after the replacement
    #[clap(long, num_args(1..))]
    pub min_nakamoto_coefficients: Vec<String>,

    /// Features or Node IDs to exclude from the available nodes pool
    #[clap(long, num_args(1..))]
    pub exclude: Vec<String>,

    /// Features or node IDs to only choose from
    #[clap(long, num_args(1..))]
    pub only: Vec<String>,

    /// Force the inclusion of the provided nodes for replacement, regardless
    /// of the decentralization score
    #[clap(long, num_args(1..))]
    pub include: Vec<PrincipalId>,

    /// The ID of the subnet.
    #[clap(long, short)]
    pub id: Option<PrincipalId>,
}

impl ExecutableCommand for Replace {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::default()
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let subnet_target = match &self.id {
            Some(id) => SubnetTarget::FromId(*id),
            _ => SubnetTarget::FromNodesIds(self.nodes.clone()),
        };

        let subnet_manager = ctx.subnet_manager().await;
        let subnet_change_response = subnet_manager
            .with_target(subnet_target)
            .membership_replace(
                !self.no_heal,
                self.motivation.clone().unwrap_or_default(),
                self.optimize,
                self.exclude.clone().into(),
                self.only.clone(),
                self.include.clone().into(),
                Self::parse_min_nakamoto_coefficients(&self.min_nakamoto_coefficients),
            )
            .await?;

        let runner = ctx.runner().await;

        runner.propose_subnet_change(subnet_change_response).await
    }

    fn validate(&self, cmd: &mut clap::Command) {
        if !self.nodes.is_empty() && self.id.is_some() {
            cmd.error(
                ErrorKind::ArgumentConflict,
                "Both subnet id and a list of nodes to replace are provided. Only one of the two is allowed.",
            )
            .exit();
        } else if self.nodes.is_empty() && self.id.is_none() {
            cmd.error(
                ErrorKind::MissingRequiredArgument,
                "Specify either a subnet id or a list of nodes to replace",
            )
            .exit();
        } else if !self.nodes.is_empty() && self.motivation.is_none() {
            cmd.error(ErrorKind::MissingRequiredArgument, "Required argument motivation not found")
                .exit();
        }

        Self::validate_min_nakamoto_coefficients(cmd, &self.min_nakamoto_coefficients);
    }
}
