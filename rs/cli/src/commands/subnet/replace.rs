use clap::{Args, error::ErrorKind};

use decentralization::network::SubnetQueryBy;
use ic_types::PrincipalId;
use itertools::Itertools;

use crate::exe::args::GlobalArgs;
use crate::forum::ForumPostKind;
use crate::submitter::{SubmissionParameters, Submitter};
use crate::{auth::AuthRequirement, exe::ExecutableCommand, subnet_manager::SubnetTarget};

#[derive(Args, Debug)]
pub struct Replace {
    /// Specific node IDs to remove from the subnet
    #[clap(long, short, num_args(1..), visible_aliases = &["nodes", "node", "node-id", "node-ids", "remove", "remove-node", "remove-nodes", "remove-node-id", "remove-node-ids"])]
    pub remove_nodes: Vec<PrincipalId>,

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
    #[clap(long, num_args(1..), visible_aliases = &["add", "add-node", "add-node-id", "add-node-ids"])]
    pub add_nodes: Vec<PrincipalId>,

    /// The ID of the subnet. Must either match the subnet of the provided nodes, or be omitted.
    #[clap(long, short, visible_aliases = &["subnet", "id"])]
    pub subnet_id: Option<PrincipalId>,

    #[clap(flatten)]
    pub submission_parameters: SubmissionParameters,
}

impl ExecutableCommand for Replace {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        self.validate_subnet_id_and_nodes(&ctx).await?;

        let subnet_target = match &self.subnet_id {
            Some(id) => SubnetTarget::FromId(*id),
            _ => SubnetTarget::FromNodesIds(self.remove_nodes.clone()),
        };

        let all_nodes = ctx.load_registry().await.nodes().await?.values().cloned().collect_vec();

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
        if self.remove_nodes.is_empty() && self.subnet_id.is_none() {
            cmd.error(
                ErrorKind::MissingRequiredArgument,
                "Specify either a subnet ID or a list of nodes to replace",
            )
            .exit()
        } else if !self.remove_nodes.is_empty() && self.motivation.is_none() {
            cmd.error(ErrorKind::MissingRequiredArgument, "Required argument motivation not found")
                .exit()
        }
    }
}

impl Replace {
    /// If both a subnet id and nodes to remove are provided, ensure they match
    async fn validate_subnet_id_and_nodes(&self, ctx: &crate::ctx::DreContext) -> anyhow::Result<()> {
        let nodes_add_or_remove = [self.remove_nodes.clone(), self.add_nodes.clone()].concat();
        if let (Some(expected_subnet_id), false) = (self.subnet_id, nodes_add_or_remove.is_empty()) {
            let registry = ctx.load_registry().await;
            let nodes = registry.get_nodes_from_ids(&nodes_add_or_remove).await?;
            let subnet = registry
                .subnet(SubnetQueryBy::NodeList(nodes))
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            if subnet.id != expected_subnet_id {
                anyhow::bail!("Provided --id does not match the subnet of --remove-nodes");
            }
        }
        Ok(())
    }
}
