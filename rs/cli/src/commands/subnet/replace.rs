use clap::{error::ErrorKind, Args};
use ic_types::PrincipalId;

use crate::{
    commands::{AuthRequirement, ExecutableCommand},
    subnet_manager::SubnetTarget,
};

#[derive(Args, Debug)]
pub struct Replace {
    /// Set of custom nodes to be replaced
    #[clap(long, short, num_args(1..), visible_aliases = &["node", "nodes", "node-id", "node-ids"])]
    pub nodes: Vec<PrincipalId>,

    /// Do not replace unhealthy nodes
    #[clap(long)]
    pub no_heal: bool,

    #[clap(long, short, help = r#"How many nodes to try replacing in the subnet to improve decentralization?"#)]
    pub optimize: Option<usize>,

    /// Motivation for replacing custom nodes
    #[clap(long, short, aliases = [ "summary" ])]
    pub motivation: Option<String>,

    /// Features or Node IDs to exclude from the available nodes pool
    #[clap(long, num_args(1..))]
    pub exclude: Vec<String>,

    /// Features or node IDs to only choose from
    #[clap(long, num_args(1..))]
    pub only: Vec<String>,

    /// Force the inclusion of the provided nodes for replacement, regardless
    /// of the decentralization coefficients
    #[clap(long, num_args(1..))]
    pub include: Vec<PrincipalId>,

    /// The ID of the subnet.
    #[clap(long, short, alias = "subnet-id")]
    pub id: Option<PrincipalId>,
}

impl ExecutableCommand for Replace {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let subnet_target = match &self.id {
            Some(id) => SubnetTarget::FromId(*id),
            _ => SubnetTarget::FromNodesIds(self.nodes.clone()),
        };

        let subnet_manager = ctx.subnet_manager().await?;
        let subnet_change_response = subnet_manager
            .with_target(subnet_target)
            .membership_replace(
                !self.no_heal,
                self.motivation.clone(),
                self.optimize,
                self.exclude.clone().into(),
                self.only.clone(),
                self.include.clone().into(),
            )
            .await?;

        let runner = ctx.runner().await?;

        if let Some(runner_proposal) = runner.propose_subnet_change(subnet_change_response, ctx.forum_post_link()).await? {
            let ic_admin = ctx.ic_admin().await?;
            ic_admin.propose_run(runner_proposal.cmd, runner_proposal.opts).await?;
        }
        Ok(())
    }

    fn validate(&self, _args: &crate::commands::Args, cmd: &mut clap::Command) {
        if !self.nodes.is_empty() && self.id.is_some() {
            cmd.error(
                ErrorKind::ArgumentConflict,
                "Both subnet id and a list of nodes to replace are provided. Only one of the two is allowed.",
            )
            .exit()
        } else if self.nodes.is_empty() && self.id.is_none() {
            cmd.error(
                ErrorKind::MissingRequiredArgument,
                "Specify either a subnet id or a list of nodes to replace",
            )
            .exit()
        } else if !self.nodes.is_empty() && self.motivation.is_none() {
            cmd.error(ErrorKind::MissingRequiredArgument, "Required argument motivation not found")
                .exit()
        }
    }
}
