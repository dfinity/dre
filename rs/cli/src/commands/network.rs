use clap::Args;
use ic_types::PrincipalId;
use itertools::Itertools;
use log::info;

use crate::{discourse_client::parse_proposal_id_from_ic_admin_response, ic_admin::ProposeOptions, runner::RunnerProposal};

use super::{AuthRequirement, ExecutableCommand};

#[derive(Args, Debug)]
#[clap(alias = "heal")]
pub struct Network {
    /// Heal the unhealthy subnets by replacing unhealthy nodes in them.
    #[clap(long)]
    pub heal: bool,

    /// Optimize the decentralization of the subnets that are not compliant with the
    /// business rules (target topology).
    #[clap(long, visible_alias = "optimize")]
    pub optimize_decentralization: bool,

    /// Ensure that at least one node of each node operator is
    /// assigned to some (any) subnet. Node will only be assigned to a subnet if
    /// this does not worsen the decentralization of the target subnet.
    #[clap(long)]
    pub ensure_operator_nodes_assigned: bool,

    /// Ensure that at least one node of each node operator is
    /// not assigned to any subnet. Node will only be unassigned from a subnet if
    /// this does not worsen the decentralization of the target subnet.
    #[clap(long)]
    pub ensure_operator_nodes_unassigned: bool,

    /// Do not try to improve or optimize the provided subnets.
    #[clap(long, num_args(1..), visible_alias = "skip-subnets")]
    pub omit_subnets: Vec<String>,

    /// Do not consider the following nodes for adding into the subnets, consider them as cordoned.
    /// All node ids with the provided substrings will be omitted.
    #[clap(long, num_args(1..), visible_alias = "cordoned-nodes")]
    pub omit_nodes: Vec<String>,

    /// Remove cordoned nodes from their subnets.
    #[clap(long)]
    pub remove_cordoned_nodes: bool,
}

impl ExecutableCommand for Network {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner().await?;
        let mut errors = vec![];
        let network_heal = self.heal || std::env::args().any(|arg| arg == "heal");

        let mut proposals = vec![];
        let mut omit_subnets = self.omit_subnets.clone();
        let mut omit_nodes = self.omit_nodes.clone();

        let update_omit_nodes = |proposals: &[RunnerProposal], omit_nodes: &mut Vec<String>| {
            omit_nodes.extend(
                proposals
                    .iter()
                    .filter_map(|proposal| match &proposal.cmd {
                        crate::ic_admin::ProposeCommand::ChangeSubnetMembership {
                            subnet_id: _,
                            node_ids_add,
                            node_ids_remove,
                        } => {
                            let mut omit_nodes = vec![];
                            omit_nodes.extend(node_ids_add.iter().map(|node_id| node_id.to_string()));
                            omit_nodes.extend(node_ids_remove.iter().map(|node_id| node_id.to_string()));
                            Some(omit_nodes)
                        }
                        crate::ic_admin::ProposeCommand::AddApiBoundaryNodes { nodes, version: _version } => {
                            let mut omit_nodes = vec![];
                            omit_nodes.extend(nodes.iter().map(|node| node.to_string()));
                            Some(omit_nodes)
                        }
                        _ => None,
                    })
                    .flatten()
                    .unique()
                    .collect::<Vec<String>>(),
            );
        };

        let update_omit_subnets = |proposals: &[RunnerProposal], omit_subnets: &mut Vec<String>| {
            omit_subnets.extend(
                proposals
                    .iter()
                    .filter_map(|proposal| match &proposal.cmd {
                        crate::ic_admin::ProposeCommand::ChangeSubnetMembership { subnet_id, .. } => Some(subnet_id.to_string()),
                        _ => None,
                    })
                    .unique()
                    .collect::<Vec<String>>(),
            );
        };

        if network_heal || self.optimize_decentralization || self.remove_cordoned_nodes {
            info!("Healing the network by replacing unhealthy nodes, removing cordoned nodes, and optimizing decentralization in subnets");
            let maybe_proposals = runner
                .network_heal(
                    ctx.forum_post_link(),
                    &omit_subnets,
                    &omit_nodes,
                    self.optimize_decentralization,
                    self.remove_cordoned_nodes,
                )
                .await;
            match maybe_proposals {
                Ok(heal_proposals) => {
                    update_omit_subnets(&heal_proposals, &mut omit_subnets);
                    update_omit_nodes(&heal_proposals, &mut omit_nodes);
                    proposals.extend(heal_proposals);
                }
                Err(e) => errors.push(DetailedError {
                    proposal: None,
                    error: anyhow::anyhow!(
                        "Failed to calculate proposals for healing of the network and they won't be submitted. Error received: {:?}",
                        e
                    ),
                }),
            }
        } else {
            info!("No network healing requested");
        }

        if self.ensure_operator_nodes_assigned {
            info!("Ensuring some operator nodes are assigned, for every node operator");
            let maybe_proposals = runner
                .network_ensure_operator_nodes_assigned(ctx.forum_post_link(), &omit_subnets, &omit_nodes)
                .await;
            match maybe_proposals {
                Ok(operator_assigned_proposals) => {
                    update_omit_subnets(&operator_assigned_proposals, &mut omit_subnets);
                    update_omit_nodes(&operator_assigned_proposals, &mut omit_nodes);
                    proposals.extend(operator_assigned_proposals);
                }
                Err(e) => errors.push(DetailedError {
                    proposal: None,
                    error: anyhow::anyhow!(
                        "Failed to calculate proposals for ensuring each operator has some nodes assigned and they won't be submitted. Error received: {:?}",
                        e
                    ),
                }),
            }
        } else {
            info!("No network ensure operator nodes assigned requested");
        }

        if self.ensure_operator_nodes_unassigned {
            info!("Ensuring some operator nodes are unassigned, for every node operator");
            let maybe_proposals = runner
                .network_ensure_operator_nodes_unassigned(ctx.forum_post_link(), &omit_subnets, &omit_nodes)
                .await;
            match maybe_proposals {
                Ok(operator_unassigned_proposals) => {
                    update_omit_subnets(&operator_unassigned_proposals, &mut omit_subnets);
                    update_omit_nodes(&operator_unassigned_proposals, &mut omit_nodes);
                    proposals.extend(operator_unassigned_proposals);
                }
                Err(e) => errors.push(DetailedError {
                    proposal: None,
                    error: anyhow::anyhow!(
                        "Failed to calculate proposals for ensuring each operator has some nodes unassigned and they won't be submitted. Error received: {:?}",
                        e
                    ),
                }),
            }
        } else {
            info!("No network ensure operator nodes unassigned requested");
        }

        if proposals.is_empty() {
            return Ok(());
        }

        let ic_admin = ctx.ic_admin().await?;
        let discourse_client = ctx.discourse_client()?;
        let is_fake_neuron = ctx.neuron().await?.is_fake_neuron();

        for proposal in proposals {
            if let crate::ic_admin::ProposeCommand::ChangeSubnetMembership { subnet_id, .. } = &proposal.cmd {
                if let Err(e) = process_proposal(&*ic_admin, &*discourse_client, &proposal, subnet_id, is_fake_neuron).await {
                    errors.push(e);
                }
            } else {
                errors.push(DetailedError {
                    proposal: Some(proposal.clone()),
                    error: anyhow::anyhow!("Expected all proposals to be of type `ChangeSubnetMembership`"),
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "All errors received:{}",
                errors.iter().enumerate().map(format_error).join("")
            ))
        }
    }

    fn validate(&self, _args: &crate::commands::Args, cmd: &mut clap::Command) {
        // At least one of the two options must be provided
        let network_heal = self.heal || std::env::args().any(|arg| arg == "heal");
        if !network_heal && !self.ensure_operator_nodes_assigned && !self.ensure_operator_nodes_unassigned && !self.remove_cordoned_nodes {
            cmd.error(
                clap::error::ErrorKind::MissingRequiredArgument,
                "At least one of '--heal' or '--ensure-operator-nodes-assigned' or '--ensure-operator-nodes-unassigned' or '--remove-cordoned-nodes' must be specified.",
            )
            .exit()
        }
    }
}

struct DetailedError {
    proposal: Option<RunnerProposal>,
    error: anyhow::Error,
}

impl From<anyhow::Error> for DetailedError {
    fn from(error: anyhow::Error) -> Self {
        DetailedError { proposal: None, error }
    }
}

fn format_error((i, detailed_error): (usize, &DetailedError)) -> String {
    format!(
        "\nError {}.:\n  - {}\n  - Error: {:?}",
        i + 1,
        detailed_error.proposal.as_ref().map_or_else(
            || "Error is linked to a calculation of proposals to be submitted".to_string(),
            |proposal| format!("Proposal: {:?}", proposal)
        ),
        detailed_error.error
    )
}

async fn process_proposal(
    ic_admin: &dyn crate::ic_admin::IcAdmin,
    discourse_client: &dyn crate::discourse_client::DiscourseClient,
    proposal: &RunnerProposal,
    subnet_id: &PrincipalId,
    is_fake_neuron: bool,
) -> Result<(), DetailedError> {
    match ic_admin
        .propose_print_and_confirm(
            proposal.cmd.clone(),
            ProposeOptions {
                forum_post_link: Some("[comment]: <> (Link will be added on actual execution)".to_string()),
                ..proposal.opts.clone()
            },
        )
        .await
    {
        Ok(false) => return Ok(()), // User chose not to proceed
        Ok(true) => {}
        Err(e) => {
            return Err(DetailedError {
                proposal: Some(proposal.clone()),
                error: anyhow::anyhow!("Error when prompting user for confirmation. Error: {:?}", e),
            });
        }
    }

    let body = match (&proposal.opts.motivation, &proposal.opts.summary) {
        (Some(motivation), None) => motivation.to_string(),
        (Some(motivation), Some(summary)) => format!("{}\nMotivation:\n{}", summary, motivation),
        (None, Some(summary)) => summary.to_string(),
        (None, None) => {
            return Err(DetailedError {
                proposal: Some(proposal.clone()),
                error: anyhow::anyhow!("Expected to have `motivation` or `summary` for this proposal"),
            });
        }
    };

    let maybe_topic = if is_fake_neuron {
        None
    } else {
        match discourse_client.create_replace_nodes_forum_post(*subnet_id, body).await {
            Ok(maybe_topic) => maybe_topic,
            Err(e) => {
                return Err(DetailedError {
                    proposal: Some(proposal.clone()),
                    error: anyhow::anyhow!("Error when creating a forum post: {:?}", e),
                });
            }
        }
    };

    let proposal_response = match ic_admin
        .propose_submit(
            proposal.cmd.clone(),
            ProposeOptions {
                forum_post_link: maybe_topic
                    .as_ref()
                    .map(|topic| topic.url.clone())
                    .or_else(|| proposal.opts.forum_post_link.clone()),
                ..proposal.opts.clone()
            },
        )
        .await
    {
        Ok(response) => response,
        Err(e) => {
            return Err(DetailedError {
                proposal: Some(proposal.clone()),
                error: anyhow::anyhow!("Error when submitting proposal: {:?}", e),
            });
        }
    };

    if let Some(topic) = maybe_topic {
        if let Err(e) = discourse_client
            .add_proposal_url_to_post(
                topic.update_id,
                parse_proposal_id_from_ic_admin_response(proposal_response).map_err(|e| DetailedError {
                    proposal: Some(proposal.clone()),
                    error: anyhow::anyhow!("Error parsing proposal id from ic_admin output: {:?}", e),
                })?,
            )
            .await
        {
            return Err(DetailedError {
                proposal: Some(proposal.clone()),
                error: anyhow::anyhow!("Error when updating forum post: {:?}", e),
            });
        }
    }

    Ok(())
}
