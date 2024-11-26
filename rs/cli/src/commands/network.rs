use clap::Args;
use itertools::Itertools;
use log::{info, warn};

use crate::{discourse_client::parse_proposal_id_from_governance_response, ic_admin::ProposeOptions, runner::RunnerProposal};

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

    /// Skip provided subnets.
    #[clap(long, num_args(1..))]
    pub skip_subnets: Vec<String>,

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

        if network_heal || self.optimize_decentralization {
            info!("Healing the network by replacing unhealthy nodes and optimizing decentralization in subnets that have unhealthy nodes");
            let maybe_proposals = runner
                .network_heal(ctx.forum_post_link(), &self.skip_subnets, self.optimize_decentralization)
                .await;
            match maybe_proposals {
                Ok(heal_proposals) => proposals.extend(heal_proposals),
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
                .network_ensure_operator_nodes_assigned(ctx.forum_post_link(), &self.skip_subnets)
                .await;
            match maybe_proposals {
                Ok(operator_assigned_proposals) => proposals.extend(operator_assigned_proposals),
                Err(e) => errors.push(DetailedError { proposal: None, error: anyhow::anyhow!("Failed to calculate proposals for ensuring each operator has some nodes assigned and they won't be submitted. Error received: {:?}", e) }),
            }
        } else {
            info!("No network ensure operator nodes assigned requested");
        }
        if self.ensure_operator_nodes_unassigned {
            info!("Ensuring some operator nodes are unassigned, for every node operator");
            let maybe_proposals = runner
                .network_ensure_operator_nodes_unassigned(ctx.forum_post_link(), &self.skip_subnets)
                .await;
            match maybe_proposals {
                Ok(operator_unassigned_proposals) => proposals.extend(operator_unassigned_proposals),
                Err(e) => errors.push(DetailedError { proposal: None, error: anyhow::anyhow!("Failed to calculate proposals for ensuring each operator has some nodes unassigned and they won't be submitted. Error received: {:?}", e) }),
            }
        } else {
            info!("No network ensure operator nodes unassigned requested");
        }

        if self.remove_cordoned_nodes {
            info!("Removing cordoned nodes from their subnets");
            let maybe_proposals = runner.network_remove_cordoned_nodes(ctx.forum_post_link(), &self.skip_subnets).await;
            match maybe_proposals {
                Ok(remove_cordoned_nodes_proposals) => proposals.extend(remove_cordoned_nodes_proposals),
                Err(e) => errors.push(DetailedError {
                    proposal: None,
                    error: anyhow::anyhow!(
                        "Failed to calculate proposals for removing cordoned nodes and they won't be submitted. Error received: {:?}",
                        e
                    ),
                }),
            }
        } else {
            info!("No network remove cordoned nodes requested");
        }

        // This check saves time if there are no proposals to be submitted
        // because it won't check for new versions of ic admin
        if !proposals.is_empty() {
            let ic_admin = ctx.ic_admin().await?;
            let discourse_client = ctx.discourse_client()?;
            for proposal in proposals {
                let subnet_id = match &proposal.cmd {
                    crate::ic_admin::ProposeCommand::ChangeSubnetMembership {
                        subnet_id,
                        node_ids_add: _,
                        node_ids_remove: _,
                    } => subnet_id,
                    _ => {
                        errors.push(DetailedError {
                            proposal: Some(proposal.clone()),
                            error: anyhow::anyhow!("Expected all proposals to be of type `ChangeSubnetMembership`"),
                        });
                        continue;
                    }
                };

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
                    Ok(should_proceed) => {
                        if !should_proceed {
                            // Said "no" to the prompt in cli, just abort this proposal
                            // but this is not an error
                            continue;
                        }
                        // Said "yes" to the prompt in cli
                    }
                    Err(e) => {
                        errors.push(DetailedError {
                            proposal: Some(proposal.clone()),
                            error: anyhow::anyhow!("Received error when prompting user for confirmation. Error: {:?}", e),
                        });
                        continue;
                    }
                }

                let body = match (&proposal.opts.motivation, &proposal.opts.summary) {
                    (Some(motivation), None) => motivation.to_string(),
                    (Some(motivation), Some(summary)) => format!("{}\nMotivation:\n{}", summary, motivation),
                    (None, Some(summary)) => summary.to_string(),
                    (None, None) => anyhow::bail!("Expected to have `motivation` or `summary` for this proposal"),
                };

                let maybe_topic = match discourse_client.create_replace_nodes_forum_post(*subnet_id, body).await {
                    Ok(maybe_topic) => maybe_topic,
                    Err(e) => {
                        errors.push(DetailedError {
                            proposal: Some(proposal.clone()),
                            error: anyhow::anyhow!("Got error when creating a forum post: {:?}", e),
                        });
                        continue;
                    }
                };

                let proposal_response = match ic_admin
                    .propose_submit(
                        proposal.cmd.clone(),
                        ProposeOptions {
                            forum_post_link: match (maybe_topic.as_ref(), proposal.opts.forum_post_link.as_ref()) {
                                (Some(discourse_response), _) => Some(discourse_response.url.clone()),
                                (None, Some(from_cli_or_auto_formated)) => Some(from_cli_or_auto_formated.clone()),
                                _ => {
                                    warn!("Didn't find a link to forum post from discourse or cli and couldn't auto-format it.");
                                    warn!("Will not add forum post to the proposal");
                                    None
                                }
                            },
                            ..proposal.opts.clone()
                        },
                    )
                    .await
                {
                    Ok(response) => response,
                    Err(e) => {
                        errors.push(DetailedError {
                            proposal: Some(proposal.clone()),
                            error: anyhow::anyhow!("Received error when submitting proposal: {:?}", e),
                        });
                        continue;
                    }
                };

                if let Some(topic) = maybe_topic {
                    if let Err(e) = discourse_client
                        .add_proposal_url_to_post(topic.update_id, parse_proposal_id_from_governance_response(proposal_response)?)
                        .await
                    {
                        errors.push(DetailedError {
                            proposal: Some(proposal.clone()),
                            error: anyhow::anyhow!("Received error when updating forum post: {:?}", e),
                        });
                    }
                }
            }
        }

        match errors.is_empty() {
            true => Ok(()),
            false => Err(anyhow::anyhow!(
                r#"All errors received:{}"#,
                errors
                    .iter()
                    .enumerate()
                    .map(|(i, detailed_error)| format!(
                        r#"
Error {}.:
  - {}
  - Error: {:?}"#,
                        i + 1,
                        match &detailed_error.proposal {
                            Some(proposal) => format!("Proposal: {:?}", proposal),
                            None => "Error is linked to a calculation of proposals to be submitted".to_string(),
                        },
                        detailed_error.error.to_string()
                    ))
                    // Doesn't need a `\n` because of the way the whole error is formatted.
                    .join("")
            )),
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
