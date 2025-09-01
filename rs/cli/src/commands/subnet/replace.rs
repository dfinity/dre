use clap::{error::ErrorKind, Args};

use ic_types::PrincipalId;
use itertools::Itertools;

use crate::exe::args::GlobalArgs;
use crate::forum::ForumPostKind;
use crate::submitter::{SubmissionParameters, Submitter};
use crate::{auth::AuthRequirement, exe::ExecutableCommand, subnet_manager::SubnetTarget};

#[derive(Args, Debug)]
pub struct Replace {
    /// Specific node IDs to remove from the subnet
    #[clap(long = "remove-nodes", short, num_args(1..), visible_aliases = &["nodes", "node", "node-id", "node-ids", "remove", "remove-node", "remove-nodes", "remove-node-id", "remove-node-ids"])]
    pub nodes: Vec<PrincipalId>,

    /// Do not replace unhealthy nodes
    #[clap(long)]
    pub no_heal: bool,

    /// Number of nodes to replace (system will pick which to optimize decentralization)
    #[clap(long = "replace-count", short, visible_aliases = &["optimize", "optimise", "optimize-count"])]
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

    /// Add specific nodes to the subnet. Fails if a node is unavailable/unhealthy.
    #[clap(long = "add-nodes", num_args(1..), visible_aliases = &["add", "add-node", "add-node-id", "add-node-ids"])]
    pub add_nodes: Vec<PrincipalId>,

    /// The ID of the subnet.
    #[clap(long, short, alias = "subnet-id")]
    pub id: Option<PrincipalId>,

    #[clap(flatten)]
    pub submission_parameters: SubmissionParameters,
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
                self.add_nodes.clone().into(),
                &all_nodes,
            )
            .await?;

        let runner_proposal = match ctx.runner().await?.propose_subnet_change(&subnet_change_response, false).await? {
            Some(runner_proposal) => runner_proposal,
            None => return Ok(()),
        };

        Submitter::from(&self.submission_parameters)
            .propose_and_print(
                ctx.ic_admin_executor().await?.execution(runner_proposal.clone()),
                match subnet_change_response.subnet_id {
                    Some(id) => ForumPostKind::ReplaceNodes {
                        subnet_id: id,
                        body: match (&runner_proposal.options.motivation, &runner_proposal.options.summary) {
                            (Some(motivation), None) => motivation.to_string(),
                            (Some(motivation), Some(summary)) => format!("{}\nMotivation:\n{}", summary, motivation),
                            (None, Some(summary)) => summary.to_string(),
                            (None, None) => anyhow::bail!("Expected to have `motivation` or `summary` for this proposal"),
                        },
                    },
                    None => ForumPostKind::Generic,
                },
            )
            .await
    }

    fn validate(&self, _args: &GlobalArgs, cmd: &mut clap::Command) {
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
