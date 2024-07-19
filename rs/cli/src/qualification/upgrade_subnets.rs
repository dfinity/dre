use std::{fmt::Display, rc::Rc, time::Duration};

use ic_management_backend::lazy_registry::LazyRegistry;
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use itertools::Itertools;
use reqwest::ClientBuilder;

use crate::{
    ctx::DreContext,
    ic_admin::{ProposeCommand, ProposeOptions},
    qualification::{
        print_table,
        tabular_util::{ColumnAlignment, Table},
    },
};

use super::{ic_admin_with_retry, print_subnet_versions, print_text, Step};

pub struct UpgradeSubnets {
    pub subnet_type: Option<SubnetType>,
    pub to_version: String,
    pub action: Action,
}

pub enum Action {
    Upgrade,
    Downgrade,
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Action::Upgrade => "upgrade".to_string(),
                Action::Downgrade => "downgrade".to_string(),
            }
        )
    }
}

impl Step for UpgradeSubnets {
    fn help(&self) -> String {
        format!(
            "This step {} all the {} to the desired version",
            self.action,
            match self.subnet_type {
                Some(s) => match s {
                    SubnetType::Application => "application subnets",
                    SubnetType::System => "system subnets",
                    SubnetType::VerifiedApplication => "verified-application subnets",
                },
                None => "unassigned nodes",
            }
        )
    }

    fn name(&self) -> String {
        format!(
            "{}_{}_version",
            self.action,
            match self.subnet_type {
                Some(s) => match s {
                    SubnetType::Application => "application_subnet",
                    SubnetType::System => "system_subnet",
                    SubnetType::VerifiedApplication => "verified-application_subnet",
                },
                None => "unassigned_nodes",
            }
        )
    }

    async fn execute(&self, ctx: &DreContext) -> anyhow::Result<()> {
        let registry = ctx.registry().await;
        let subnets = registry.subnets().await?;
        print_text(format!("Found total of {} nodes", registry.nodes().await?.len()));
        print_subnet_versions(registry.clone()).await?;

        if let Some(subnet_type) = &self.subnet_type {
            for subnet in subnets
                .values()
                .filter(|s| s.subnet_type.eq(subnet_type) && !s.replica_version.eq(&self.to_version))
            {
                let registry = ctx.registry().await;
                print_text(format!(
                    "Upgrading subnet {}: {} -> {}",
                    subnet.principal, &subnet.replica_version, &self.to_version
                ));

                // Place proposal
                ic_admin_with_retry(
                    ctx.ic_admin(),
                    ProposeCommand::DeployGuestosToAllSubnetNodes {
                        subnet: subnet.principal,
                        version: self.to_version.clone(),
                    },
                    ProposeOptions {
                        title: Some(format!("Propose to upgrade subnet {} to {}", subnet.principal, &self.to_version)),
                        summary: Some("Qualification testing".to_string()),
                        motivation: Some("Qualification testing".to_string()),
                    },
                )
                .await?;

                print_text(format!("Placed proposal for subnet {}", subnet.principal));

                // Wait for the version to be active on the subnet
                wait_for_subnet_revision(registry.clone(), Some(subnet.principal), &self.to_version).await?;

                print_text(format!(
                    "Subnet {} successfully upgraded to version {}",
                    subnet.principal, &self.to_version
                ));

                print_subnet_versions(registry.clone()).await?;
            }
        } else {
            let registry = ctx.registry().await;
            let unassigned_nodes_version = registry.unassigned_nodes_replica_version()?;
            if unassigned_nodes_version.to_string() == self.to_version {
                print_text(format!("Unassigned nodes are already on {}, skipping", self.to_version));
                return Ok(());
            }
            print_text(format!(
                "Upgrading unassigned version: {} -> {}",
                &unassigned_nodes_version, &self.to_version
            ));

            ic_admin_with_retry(
                ctx.ic_admin(),
                ProposeCommand::DeployGuestosToAllUnassignedNodes {
                    replica_version: self.to_version.clone(),
                },
                ProposeOptions {
                    title: Some("Upgrading unassigned nodes".to_string()),
                    summary: Some("Upgrading unassigned nodes".to_string()),
                    motivation: Some("Upgrading unassigned nodes".to_string()),
                },
            )
            .await?;

            wait_for_subnet_revision(registry.clone(), None, &self.to_version).await?;

            print_text(format!("Unassigned nodes successfully upgraded to version {}", &self.to_version));

            print_subnet_versions(registry.clone()).await?;
        }

        Ok(())
    }
}

const MAX_TRIES: usize = 100;
const SLEEP: Duration = Duration::from_secs(10);
const TIMEOUT: Duration = Duration::from_secs(60);
const PLACEHOLDER: &str = "upgrading...";

async fn wait_for_subnet_revision(registry: Rc<LazyRegistry>, subnet: Option<PrincipalId>, revision: &str) -> anyhow::Result<()> {
    let client = ClientBuilder::new().connect_timeout(TIMEOUT).build()?;
    for i in 0..MAX_TRIES {
        tokio::time::sleep(SLEEP).await;
        print_text(format!(
            "- {} - Checking if {} on {}",
            i,
            match &subnet {
                Some(p) => format!("{} subnet is", p),
                None => "unassigned nodes are".to_string(),
            },
            revision
        ));

        if let Err(e) = registry.sync_with_nns().await {
            print_text(format!("Received error when syncing registry: {}", e));
            continue;
        }

        // Fetch the nodes of the subnet
        let nodes = registry.nodes().await?;
        let nodes = nodes.values().filter(|n| n.subnet_id.eq(&subnet)).collect_vec();

        let mut nodes_with_reivison = vec![];
        // Fetch the metrics of each node and check if it
        // contains the revision somewhere
        for node in nodes {
            let url = format!("http://[{}]:9090/metrics", node.ip_addr);

            let response = match client.get(&url).send().await {
                Ok(r) => match r.error_for_status() {
                    Ok(r) => match r.text().await {
                        Ok(r) => r,
                        Err(e) => {
                            print_text(format!("Received error {}, skipping...", e));
                            continue;
                        }
                    },
                    Err(e) => {
                        print_text(format!("Received error {}, skipping...", e));
                        continue;
                    }
                },
                Err(e) => {
                    print_text(format!("Received error {}, skipping...", e));
                    continue;
                }
            };
            if response.contains(revision) {
                nodes_with_reivison.push((node.principal.to_string(), revision));
                continue;
            }
            nodes_with_reivison.push((node.principal.to_string(), PLACEHOLDER));
        }

        // print the status of nodes and versions
        let table = Table::new()
            .with_columns(&[("Node Id", ColumnAlignment::Middle), ("Revision", ColumnAlignment::Left)])
            .with_rows(
                nodes_with_reivison
                    .iter()
                    .map(|(pr, v)| vec![pr.to_string(), v.to_string()])
                    .collect_vec(),
            )
            .to_table();
        print_table(table);

        // Check if done
        if !nodes_with_reivison.iter().any(|(_, r)| *r == PLACEHOLDER) {
            return Ok(());
        }
    }

    anyhow::bail!(
        "Maximum number of retires reached and the revision is not empty on all nodes in the subnet {}",
        subnet.map(|p| p.to_string()).unwrap_or("of unassigned nodes".to_string())
    )
}
