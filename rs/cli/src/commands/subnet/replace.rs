use clap::{error::ErrorKind, Args};

use ic_types::PrincipalId;
use itertools::Itertools;
use log::warn;

use crate::{
    commands::{AuthRequirement, ExecutableCommand},
    discourse_client::parse_proposal_id_from_ic_admin_response,
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
        if let Some(runner_proposal) = runner.propose_subnet_change(subnet_change_response, ctx.forum_post_link()).await? {
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

            let discourse_client = ctx.discourse_client()?;
            let maybe_topic = if let Some(id) = subnet_id {
                let body = match (&runner_proposal.opts.motivation, &runner_proposal.opts.summary) {
                    (Some(motivation), None) => motivation.to_string(),
                    (Some(motivation), Some(summary)) => format!("{}\nMotivation:\n{}", summary, motivation),
                    (None, Some(summary)) => summary.to_string(),
                    (None, None) => anyhow::bail!("Expected to have `motivation` or `summary` for this proposal"),
                };

                discourse_client.create_replace_nodes_forum_post(id, body).await?
            } else {
                None
            };

            let proposal_response = ic_admin
                .propose_submit(
                    runner_proposal.cmd,
                    ProposeOptions {
                        forum_post_link: match (maybe_topic.as_ref(), runner_proposal.opts.forum_post_link.as_ref()) {
                            (Some(discourse_response), _) => Some(discourse_response.url.clone()),
                            (None, Some(from_cli_or_auto_formated)) => Some(from_cli_or_auto_formated.clone()),
                            _ => {
                                warn!("Didn't find a link to forum post from discourse or cli and couldn't auto-format it.");
                                warn!("Will not add forum post to the proposal");
                                None
                            }
                        },
                        ..runner_proposal.opts
                    },
                )
                .await?;

            if let Some(topic) = maybe_topic {
                discourse_client
                    .add_proposal_url_to_post(topic.update_id, parse_proposal_id_from_ic_admin_response(proposal_response)?)
                    .await?
            }
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
