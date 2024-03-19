use std::time::Duration;

use dre::{
    cli::Opts,
    ic_admin::{IcAdminWrapper, ProposeCommand, ProposeOptions},
};
use ic_base_types::PrincipalId;
use ic_management_types::Network;
use slog::{info, Logger};

#[derive(Debug)]
pub enum SubnetAction {
    Noop {
        subnet_short: String,
    },
    Baking {
        subnet_short: String,
        remaining: Duration,
    },
    PendingProposal {
        subnet_short: String,
        proposal_id: u64,
    },
    PlaceProposal {
        is_unassigned: bool,
        subnet_principal: PrincipalId,
        version: String,
    },
    WaitForNextWeek {
        subnet_short: String,
    },
}

impl<'a> SubnetAction {
    fn execute(&self, executor: &'a ActionExecutor) -> anyhow::Result<()> {
        match self {
            SubnetAction::Noop { subnet_short } => {
                if let Some(logger) = executor.logger {
                    info!(logger, "Noop for subnet '{}'", subnet_short)
                }
            }
            SubnetAction::Baking {
                subnet_short,
                remaining,
            } => {
                let humantime = humantime::format_duration(*remaining);
                if let Some(logger) = executor.logger {
                    info!(logger, "Subnet '{}' is pending to bake for {}", subnet_short, humantime)
                }
            }
            SubnetAction::PendingProposal {
                subnet_short,
                proposal_id,
            } => {
                if let Some(logger) = executor.logger {
                    info!(
                        logger,
                        "Subnet '{}' has a pending proposal with id '{}' that has to be voted on",
                        subnet_short,
                        proposal_id
                    )
                }
            }
            SubnetAction::WaitForNextWeek { subnet_short } => {
                if let Some(logger) = executor.logger {
                    info!(logger, "Waiting for next week to place proposal for '{}'", subnet_short)
                }
            }
            SubnetAction::PlaceProposal {
                is_unassigned,
                subnet_principal,
                version,
            } => {
                let principal_string = subnet_principal.to_string();
                if let Some(logger) = executor.logger {
                    info!(
                        logger,
                        "Placing proposal for '{}' to upgrade to version '{}'",
                        match is_unassigned {
                            true => "unassigned nodes",
                            false => principal_string.as_str(),
                        },
                        version
                    )
                }

                let proposal = match is_unassigned {
                    true => ProposeCommand::UpdateUnassignedNodes {
                        replica_version: version.to_string(),
                    },
                    false => ProposeCommand::UpdateSubnetReplicaVersion {
                        subnet: *subnet_principal,
                        version: version.to_string(),
                    },
                };

                let opts = ProposeOptions {
                    title: Some(format!(
                        "Update subnet {} to replica version {}",
                        principal_string.split_once('-').expect("Should contain '-'").0,
                        version.split_at(8).0
                    )),
                    summary: Some(format!(
                        "Update subnet {} to replica version {}",
                        principal_string, version
                    )),
                    ..Default::default()
                };

                executor.ic_admin.propose_run(proposal, opts, executor.simulate)?;
            }
        }

        Ok(())
    }
}

pub struct ActionExecutor<'a> {
    ic_admin: IcAdminWrapper,
    simulate: bool,
    logger: Option<&'a Logger>,
}

impl<'a> ActionExecutor<'a> {
    pub async fn new(
        neuron_id: u64,
        private_key_pem: String,
        network: Network,
        simulate: bool,
        logger: Option<&'a Logger>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            ic_admin: dre::cli::Cli::from_opts(
                &Opts {
                    neuron_id: Some(neuron_id),
                    private_key_pem: Some(private_key_pem),
                    yes: true,
                    network,
                    ..Default::default()
                },
                true,
            )
            .await?
            .into(),
            simulate,
            logger,
        })
    }

    pub async fn test(network: Network, logger: Option<&'a Logger>) -> anyhow::Result<Self> {
        Ok(Self {
            ic_admin: dre::cli::Cli::from_opts(
                &Opts {
                    yes: true,
                    network,
                    ..Default::default()
                },
                false,
            )
            .await?
            .into(),
            simulate: true,
            logger,
        })
    }

    pub fn execute(&self, actions: Vec<SubnetAction>) -> anyhow::Result<()> {
        if let Some(logger) = self.logger {
            info!(logger, "Executing following actions: {:?}", actions)
        }

        for (i, action) in actions.iter().enumerate() {
            if let Some(logger) = self.logger {
                info!(logger, "Executing action {}: {:?}", i, action)
            }
            action.execute(self)?;
        }

        Ok(())
    }
}
