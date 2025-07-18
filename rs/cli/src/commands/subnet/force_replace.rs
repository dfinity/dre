use std::collections::BTreeSet;

use clap::Args;
use ic_management_types::Node;
use ic_types::PrincipalId;
use indexmap::IndexMap;
use itertools::Itertools;
use registry_canister::mutations::do_change_subnet_membership::ChangeSubnetMembershipPayload;

use crate::{
    exe::ExecutableCommand,
    forum::ForumPostKind,
    submitter::{SubmissionParameters, Submitter},
};

#[derive(Args, Debug)]
pub struct ForceReplace {
    /// Subnet id to perform force replacement from
    #[clap(long)]
    subnet_id: PrincipalId,

    /// Nodes to remove from the given subnet
    #[clap(long, num_args = 1..)]
    from: Vec<PrincipalId>,

    /// Nodes to include into a given subnet
    #[clap(long, num_args = 1..)]
    to: Vec<PrincipalId>,

    /// Additional motivation
    #[clap(long)]
    motivation: Option<String>,

    #[clap(flatten)]
    pub submission_parameters: SubmissionParameters,
}

impl ExecutableCommand for ForceReplace {
    fn require_auth(&self) -> crate::auth::AuthRequirement {
        crate::auth::AuthRequirement::Neuron
    }

    fn validate(&self, _args: &crate::exe::args::GlobalArgs, cmd: &mut clap::Command) {
        let from: BTreeSet<PrincipalId> = self.from.iter().cloned().collect();
        let to: BTreeSet<PrincipalId> = self.to.iter().cloned().collect();

        if from.len() != to.len() {
            cmd.error(
                clap::error::ErrorKind::InvalidValue,
                "`from` and `to` have to contain the same number of elements".to_string(),
            )
            .exit();
        }

        let duplicates = from.intersection(&to).collect_vec();

        if duplicates.is_empty() {
            return;
        }

        let duplicates = duplicates.iter().map(|p| p.to_string().split_once("-").unwrap().0.to_string()).join(", ");

        cmd.error(
            clap::error::ErrorKind::ValueValidation,
            format!("`from` and `to` contain the following duplicates: [{duplicates}]"),
        )
        .exit()
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let registry = ctx.registry().await;

        let subnets = registry.subnets().await?;
        let subnet = subnets
            .get(&self.subnet_id)
            .ok_or_else(|| anyhow::anyhow!("Subnet {} is not present in the registry.", self.subnet_id))?;

        // Ensure that the `from` nodes are in the subnet
        let wrong_from_nodes: Vec<&PrincipalId> = self
            .from
            .iter()
            .filter(|p| !subnet.nodes.iter().any(|node| node.principal == **p))
            .collect();

        if !wrong_from_nodes.is_empty() {
            return Err(anyhow::anyhow!(
                "The following nodes are not members of subnet {}: [{}]",
                self.subnet_id,
                wrong_from_nodes.iter().map(|p| p.to_string()).join(", ")
            ));
        }

        // Ensure that the `to` nodes are not in any subnet
        let nodes = registry.nodes().await?;
        let unassigned_nodes: IndexMap<PrincipalId, Node> = nodes
            .iter()
            .filter(|(_, n)| n.subnet_id.is_none())
            .map(|(k, v)| (*k, v.clone()))
            .collect();

        let wrong_to_nodes: Vec<&PrincipalId> = self.to.iter().filter(|p| !unassigned_nodes.contains_key(*p)).collect();

        if !wrong_to_nodes.is_empty() {
            return Err(anyhow::anyhow!(
                "The following nodes are not found in unassigned nodes: [{}]",
                wrong_to_nodes.iter().map(|p| p.to_string()).join(", ")
            ));
        }

        // Check that no included nodes have open proposals.
        let nodes_with_proposals: Vec<Node> = self
            .from
            .iter()
            .chain(self.to.iter())
            .map(|node| nodes.get(node).unwrap())
            .filter(|node| node.proposal.is_some())
            .cloned()
            .collect();

        if !nodes_with_proposals.is_empty() {
            return Err(anyhow::anyhow!(
                "The following nodes have open proposals:\n{}",
                nodes_with_proposals
                    .iter()
                    .map(|node| format!(" - {} - {}", node.principal, node.proposal.clone().unwrap().id))
                    .join("\n")
            ));
        }

        // Create a request
        let runner = ctx.runner().await?;

        let change_membership = ChangeSubnetMembershipPayload {
            subnet_id: self.subnet_id,
            node_ids_add: self.to.iter().map(|id| (*id).into()).collect(),
            node_ids_remove: self.from.iter().map(|id| (*id).into()).collect(),
        };

        let subnet_change_response = runner.decentralization_change(&change_membership, None, self.motivation.clone()).await?;

        let runner_proposal = match ctx.runner().await?.propose_subnet_change(&subnet_change_response).await? {
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
}
