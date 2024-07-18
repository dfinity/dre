use std::{rc::Rc, time::Duration};

use ic_management_backend::lazy_registry::LazyRegistry;
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use itertools::Itertools;
use reqwest::ClientBuilder;

use crate::{
    ic_admin::{ProposeCommand, ProposeOptions},
    qualification::{
        print_table,
        tabular_util::{ColumnAlignment, Table},
    },
};

use super::{print_subnet_versions, print_text, Step};

pub struct UpgradeSubnets {
    pub subnet_type: SubnetType,
    pub to_version: String,
    pub action: Action,
}

pub enum Action {
    Upgrade,
    Downgrade,
}

impl ToString for Action {
    fn to_string(&self) -> String {
        match self {
            Action::Upgrade => "upgrade".to_string(),
            Action::Downgrade => "downgrade".to_string(),
        }
    }
}

impl Step for UpgradeSubnets {
    fn help(&self) -> String {
        format!(
            "This step {} all the {} subnets to the desired version",
            self.action.to_string(),
            match self.subnet_type {
                SubnetType::Application => "application",
                SubnetType::System => "system",
                SubnetType::VerifiedApplication => "verified-application",
            }
        )
    }

    fn name(&self) -> String {
        format!(
            "{}_{}_subnet_version",
            self.action.to_string(),
            match self.subnet_type {
                SubnetType::Application => "application",
                SubnetType::System => "system",
                SubnetType::VerifiedApplication => "verified-application",
            }
        )
    }

    async fn execute(&self, ctx: &super::QualificationContext) -> anyhow::Result<()> {
        let registry = ctx.dre_ctx.registry().await;
        let subnets = registry.subnets().await?;
        print_text(format!("Found total of {} nodes", registry.nodes().await?.len()));
        print_subnet_versions(registry.clone()).await?;

        for subnet in subnets
            .values()
            .filter(|s| s.subnet_type.eq(&self.subnet_type) && !s.replica_version.eq(&self.to_version))
        {
            let registry = ctx.dre_ctx.registry().await;
            print_text(format!(
                "Upgrading subnet {}: {} -> {}",
                subnet.principal.to_string(),
                &subnet.replica_version,
                &self.to_version
            ));

            // Place proposal
            let ic_admin = ctx.dre_ctx.ic_admin();
            ic_admin
                .propose_run(
                    ProposeCommand::DeployGuestosToAllSubnetNodes {
                        subnet: subnet.principal.clone(),
                        version: self.to_version.clone(),
                    },
                    ProposeOptions {
                        title: Some(format!(
                            "Propose to upgrade subnet {} to {}",
                            subnet.principal.to_string(),
                            &self.to_version
                        )),
                        summary: Some("Qualification testing".to_string()),
                        motivation: Some("Qualification testing".to_string()),
                    },
                )
                .await?;
            print_text(format!("Placed proposal for subnet {}", subnet.principal.to_string()));

            // Wait for the version to be active on the subnet
            wait_for_subnet_revision(registry.clone(), subnet.principal.clone(), &self.to_version).await?;

            print_text(format!(
                "Subnet {} successfully upgraded to version {}",
                subnet.principal.to_string(),
                &self.to_version
            ));

            print_subnet_versions(registry.clone()).await?;
        }

        Ok(())
    }

    async fn print_status(&self, _ctx: &super::QualificationContext) -> anyhow::Result<()> {
        Ok(())
    }
}

const MAX_TRIES: usize = 100;
const SLEEP: Duration = Duration::from_secs(10);
const TIMEOUT: Duration = Duration::from_secs(60);
const PLACEHOLDER: &str = "upgrading...";

async fn wait_for_subnet_revision(registry: Rc<LazyRegistry>, subnet: PrincipalId, revision: &str) -> anyhow::Result<()> {
    let client = ClientBuilder::new().connect_timeout(TIMEOUT).build()?;
    for i in 0..MAX_TRIES {
        tokio::time::sleep(SLEEP).await;
        print_text(format!("- {} - Checking if subnet {} is on {}", i, subnet.to_string(), revision));

        registry.sync_with_nns().await?;

        // Fetch the nodes of the subnet
        let nodes = registry.nodes().await?;
        let nodes = nodes.values().filter(|n| n.subnet_id.eq(&Some(subnet))).collect_vec();

        let mut nodes_with_reivison = vec![];
        // Fetch the metrics of each node and check if it
        // contains the revision somewhere
        for node in nodes {
            let url = format!("http://[{}]:9090/metrics", node.ip_addr.to_string());

            let response = match client.get(&url).send().await {
                Ok(r) => match r.error_for_status() {
                    Ok(r) => match r.text().await {
                        Ok(r) => r,
                        Err(e) => {
                            print_text(format!("Received error {}, skipping...", e.to_string()));
                            continue;
                        }
                    },
                    Err(e) => {
                        print_text(format!("Received error {}, skipping...", e.to_string()));
                        continue;
                    }
                },
                Err(e) => {
                    print_text(format!("Received error {}, skipping...", e.to_string()));
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
        subnet.to_string()
    )
}
