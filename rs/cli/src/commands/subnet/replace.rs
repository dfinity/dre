use clap::{error::ErrorKind, Args};

use ic_types::PrincipalId;
use itertools::Itertools;

use crate::{
    commands::{AuthRequirement, ExecutableCommand},
    forum::{ic_admin::forum_enabled_proposer, ForumParameters, ForumPostKind},
    ic_admin::ProposeOptions,
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

    #[clap(flatten)]
    pub forum_parameters: ForumParameters,
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

        let all_nodes = ctx.registry().await.nodes().await?.values().cloned().collect_vec();

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
                &all_nodes,
            )
            .await?;

        let runner = ctx.runner().await?;

        let subnet_id = subnet_change_response.subnet_id;
        // Should be refactored to not require forum post links like this.

        let runner_proposal = match runner.propose_subnet_change(subnet_change_response, None).await? {
            Some(runner_proposal) => runner_proposal,
            None => return Ok(()),
        };

        let ic_admin = ctx.ic_admin().await?;
        if !ic_admin
            .propose_print_and_confirm(
                runner_proposal.cmd.clone(),
                ProposeOptions {
                    forum_post_link: Some("[comment]: <> (Link will be added on actual execution)".to_string()),
                    ..runner_proposal.opts.clone()
                },
            )
            .await?
        {
            return Ok(());
        }

        let proxy = forum_enabled_proposer(&self.forum_parameters, &ctx, ic_admin);
        match subnet_id {
            Some(id) => {
                proxy
                    .propose_without_confirmation(
                        runner_proposal.cmd,
                        runner_proposal.opts.clone(),
                        ForumPostKind::ReplaceNodes {
                            subnet_id: id,
                            body: match (&runner_proposal.opts.motivation, &runner_proposal.opts.summary) {
                                (Some(motivation), None) => motivation.to_string(),
                                (Some(motivation), Some(summary)) => format!("{}\nMotivation:\n{}", summary, motivation),
                                (None, Some(summary)) => summary.to_string(),
                                (None, None) => anyhow::bail!("Expected to have `motivation` or `summary` for this proposal"),
                            },
                        },
                    )
                    .await
            }
            None => {
                proxy
                    .propose_without_confirmation(runner_proposal.cmd, runner_proposal.opts, ForumPostKind::Generic)
                    .await
            }
        }
    }

    fn validate(&self, _args: &crate::commands::Args, cmd: &mut clap::Command) {
        if !self.nodes.is_empty() && self.id.is_some() {
            cmd.error(
                ErrorKind::ArgumentConflict,
                "Both subnet ID and a list of nodes to replace are provided. Only one of the two is allowed.",
            )
            .exit()
        } else if self.nodes.is_empty() && self.id.is_none() {
            cmd.error(
                ErrorKind::MissingRequiredArgument,
                "Specify either a subnet ID or a list of nodes to replace",
            )
            .exit()
        } else if !self.nodes.is_empty() && self.motivation.is_none() {
            cmd.error(ErrorKind::MissingRequiredArgument, "Required argument motivation not found")
                .exit()
        }
    }
}
