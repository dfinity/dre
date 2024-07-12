use std::time::Duration;

use dre::{
    auth::Neuron,
    ic_admin::{IcAdminWrapper, ProposeCommand, ProposeOptions},
};
use ic_base_types::PrincipalId;
use ic_management_types::Network;
use slog::{info, Logger};

#[derive(Debug, PartialEq, Eq, Clone)]
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

impl SubnetAction {
    fn print(&self) -> String {
        match self {
            SubnetAction::Noop { subnet_short } => format!("Noop for subnet '{}'", subnet_short),
            SubnetAction::Baking { subnet_short, remaining } => {
                let humantime = humantime::format_duration(*remaining);
                format!("Subnet '{}' is pending to bake for {}", subnet_short, humantime)
            }
            SubnetAction::PendingProposal { subnet_short, proposal_id } => format!(
                "Subnet '{}' has a pending proposal with id '{}' that has to be voted on",
                subnet_short, proposal_id
            ),
            SubnetAction::PlaceProposal {
                is_unassigned,
                subnet_principal,
                version,
            } => format!(
                "Placing proposal for '{}' to upgrade to version '{}'",
                match is_unassigned {
                    true => "unassigned nodes".to_string(),
                    false => subnet_principal.to_string(),
                },
                version
            ),
            SubnetAction::WaitForNextWeek { subnet_short } => {
                format!("Waiting for next week to place proposal for '{}'", subnet_short)
            }
        }
    }
}

impl<'a> SubnetAction {
    async fn execute(&self, executor: &'a ActionExecutor<'_>, blessed_replica_versions: &'a [String]) -> anyhow::Result<()> {
        if let Some(logger) = executor.logger {
            info!(logger, "Subnet action: {}", self.print())
        }
        if let SubnetAction::PlaceProposal {
            is_unassigned,
            subnet_principal,
            version,
        } = self
        {
            if !blessed_replica_versions.contains(version) {
                return Err(anyhow::anyhow!("GuestOS version '{}' is not elected.", version));
            }
            let principal_string = subnet_principal.to_string();

            let proposal = match is_unassigned {
                true => ProposeCommand::DeployGuestosToAllUnassignedNodes {
                    replica_version: version.to_string(),
                },
                false => ProposeCommand::DeployGuestosToAllSubnetNodes {
                    subnet: *subnet_principal,
                    version: version.to_string(),
                },
            };

            let opts = ProposeOptions {
                title: Some(format!(
                    "Update subnet {} to GuestOS version {}",
                    principal_string.split_once('-').expect("Should contain '-'").0,
                    version.split_at(8).0
                )),
                summary: Some(format!("Update subnet {} to GuestOS version {}", principal_string, version)),
                ..Default::default()
            };

            executor.ic_admin_wrapper.propose_run(proposal, opts, executor.simulate).await?;
        }

        Ok(())
    }
}

pub struct ActionExecutor<'a> {
    ic_admin_wrapper: IcAdminWrapper,
    simulate: bool,
    logger: Option<&'a Logger>,
}

impl<'a> ActionExecutor<'a> {
    pub async fn new(neuron_id: u64, private_key_pem: String, network: Network, simulate: bool, logger: Option<&'a Logger>) -> anyhow::Result<Self> {
        let neuron = Neuron::new(&network, Some(neuron_id), Some(private_key_pem), None, None, None).await;
        Ok(Self {
            ic_admin_wrapper: IcAdminWrapper::new(network, None, true, neuron),
            simulate,
            logger,
        })
    }

    pub async fn test(network: Network, logger: Option<&'a Logger>) -> anyhow::Result<Self> {
        let neuron = Neuron::new(&network, None, None, None, None, None).await;
        Ok(Self {
            ic_admin_wrapper: IcAdminWrapper::new(network, None, true, neuron),
            simulate: true,
            logger,
        })
    }

    pub async fn execute(&self, actions: &[SubnetAction], blessed_replica_versions: &[String]) -> anyhow::Result<()> {
        if let Some(logger) = self.logger {
            info!(logger, "Executing following actions: {:?}", actions)
        }

        for (i, action) in actions.iter().enumerate() {
            if let Some(logger) = self.logger {
                info!(logger, "Executing action {}: {:?}", i, action)
            }
            action.execute(self, blessed_replica_versions).await?;
        }

        Ok(())
    }
}
