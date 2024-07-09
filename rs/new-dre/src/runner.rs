use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use decentralization::network::SubnetQueryBy;
use decentralization::network::TopologyManager;
use decentralization::SubnetChangeResponse;
use ic_management_backend::git_ic_repo::IcRepo;
use ic_management_backend::lazy_git::LazyGit;
use ic_management_backend::lazy_registry::LazyRegistry;
use ic_management_backend::proposal::ProposalAgent;
use ic_management_backend::registry::ReleasesOps;
use ic_management_types::Artifact;
use ic_management_types::Network;
use ic_management_types::NetworkError;
use ic_management_types::Release;
use ic_types::PrincipalId;
use itertools::Itertools;
use log::info;

use crate::ic_admin::{self, IcAdminWrapper};
use crate::ic_admin::{ProposeCommand, ProposeOptions};

pub struct Runner {
    ic_admin: Rc<IcAdminWrapper>,
    registry: Rc<LazyRegistry>,
    ic_repo: RefCell<Option<Rc<LazyGit>>>,
    network: Network,
    proposal_agent: ProposalAgent,
}

impl Runner {
    pub fn new(ic_admin: Rc<IcAdminWrapper>, registry: Rc<LazyRegistry>, network: Network, agent: ProposalAgent) -> Self {
        Self {
            ic_admin,
            registry,
            ic_repo: RefCell::new(None),
            network,
            proposal_agent: agent,
        }
    }

    fn ic_repo(&self) -> Rc<LazyGit> {
        if let Some(ic_repo) = self.ic_repo.borrow().as_ref() {
            return ic_repo.clone();
        }

        let ic_repo = Rc::new(
            LazyGit::new(
                self.network.clone(),
                self.registry
                    .elected_guestos()
                    .expect("Should be able to fetch elected guestos versions")
                    .to_vec(),
                self.registry
                    .elected_hostos()
                    .expect("Should be able to fetch elected hostos versions")
                    .to_vec(),
            )
            .expect("Should be able to create IC repo"),
        );
        *self.ic_repo.borrow_mut() = Some(ic_repo.clone());
        ic_repo
    }

    pub async fn deploy(&self, subnet: &PrincipalId, version: &str) -> anyhow::Result<()> {
        let _ = self
            .ic_admin
            .propose_run(
                ProposeCommand::DeployGuestosToAllSubnetNodes {
                    subnet: *subnet,
                    version: version.to_owned(),
                },
                ProposeOptions {
                    title: format!("Update subnet {subnet} to GuestOS version {version}").into(),
                    summary: format!("Update subnet {subnet} to GuestOS version {version}").into(),
                    motivation: None,
                },
            )
            .await?;

        Ok(())
    }

    pub async fn subnet_resize(
        &self,
        request: ic_management_types::requests::SubnetResizeRequest,
        motivation: String,
        verbose: bool,
    ) -> anyhow::Result<()> {
        let change = self
            .registry
            .modify_subnet_nodes(SubnetQueryBy::SubnetId(request.subnet))
            .await?
            .excluding_from_available(request.exclude.clone().unwrap_or_default())
            .including_from_available(request.only.clone().unwrap_or_default())
            .including_from_available(request.include.clone().unwrap_or_default())
            .resize(request.add, request.remove)?;

        let change = SubnetChangeResponse::from(&change);

        if verbose {
            if let Some(run_log) = &change.run_log {
                println!("{}\n", run_log.join("\n"));
            }
        }
        println!("{}", change);

        if change.added.is_empty() && change.removed.is_empty() {
            return Ok(());
        }
        if change.added.len() == change.removed.len() {
            self.run_membership_change(change.clone(), replace_proposal_options(&change)?).await
        } else {
            let action = if change.added.len() < change.removed.len() {
                "Removing nodes from"
            } else {
                "Adding nodes to"
            };
            self.run_membership_change(
                change,
                ProposeOptions {
                    title: format!("{action} subnet {}", request.subnet).into(),
                    summary: format!("{action} subnet {}", request.subnet).into(),
                    motivation: motivation.clone().into(),
                },
            )
            .await
        }
    }
    pub async fn subnet_create(
        &self,
        request: ic_management_types::requests::SubnetCreateRequest,
        motivation: String,
        verbose: bool,
        replica_version: Option<String>,
        other_args: Vec<String>,
        help_other_args: bool,
    ) -> anyhow::Result<()> {
        if help_other_args {
            println!("The following additional arguments are available for the `subnet create` command:");
            println!("{}", self.ic_admin.grep_subcommand_arguments("propose-to-create-subnet"));
            return Ok(());
        }

        let subnet_creation_data = self
            .registry
            .create_subnet(
                request.size,
                request.min_nakamoto_coefficients.clone(),
                request.include.clone().unwrap_or_default(),
                request.exclude.clone().unwrap_or_default(),
                request.only.clone().unwrap_or_default(),
            )
            .await?;
        let subnet_creation_data = SubnetChangeResponse::from(&subnet_creation_data);

        if verbose {
            if let Some(run_log) = &subnet_creation_data.run_log {
                println!("{}\n", run_log.join("\n"));
            }
        }
        println!("{}", subnet_creation_data);
        let replica_version = replica_version.unwrap_or(
            self.registry
                .nns_replica_version()
                .expect("Failed to get a GuestOS version of the NNS subnet")
                .expect("Failed to get a GuestOS version of the NNS subnet"),
        );

        self.ic_admin
            .propose_run(
                ProposeCommand::CreateSubnet {
                    node_ids: subnet_creation_data.added,
                    replica_version,
                    other_args,
                },
                ProposeOptions {
                    title: Some("Creating new subnet".into()),
                    summary: Some("# Creating new subnet with nodes: ".into()),
                    motivation: Some(motivation.clone()),
                },
            )
            .await?;
        Ok(())
    }

    pub async fn propose_subnet_change(&self, change: SubnetChangeResponse, verbose: bool) -> anyhow::Result<()> {
        if verbose {
            if let Some(run_log) = &change.run_log {
                println!("{}\n", run_log.join("\n"));
            }
        }
        println!("{}", change);

        if change.added.is_empty() && change.removed.is_empty() {
            return Ok(());
        }
        self.run_membership_change(change.clone(), replace_proposal_options(&change)?).await
    }

    pub async fn retireable_versions(&self, artifact: &Artifact) -> anyhow::Result<Vec<Release>> {
        match artifact {
            Artifact::GuestOs => self.retireable_guestos_versions().await,
            Artifact::HostOs => self.retireable_hostos_versions().await,
        }
    }

    async fn retireable_hostos_versions(&self) -> anyhow::Result<Vec<Release>> {
        let ic_repo = self.ic_repo();
        let hosts = ic_repo.hostos_releases().await?;
        let active_releases = hosts.get_active_branches();
        let hostos_versions: BTreeSet<String> = self.registry.nodes()?.values().map(|s| s.hostos_version.clone()).collect();
        let versions_in_proposals: BTreeSet<String> = self
            .proposal_agent
            .list_open_elect_hostos_proposals()
            .await?
            .iter()
            .flat_map(|p| p.versions_unelect.iter())
            .cloned()
            .collect();
        info!("Active releases: {}", active_releases.iter().join(", "));
        info!("HostOS versions in use on nodes: {}", hostos_versions.iter().join(", "));
        info!("HostOS versions in open proposals: {}", versions_in_proposals.iter().join(", "));
        let hostos_releases = ic_repo.hostos_releases().await?;
        Ok(hostos_releases
            .releases
            .clone()
            .into_iter()
            .filter(|rr| !active_releases.contains(&rr.branch))
            .filter(|rr| !hostos_versions.contains(&rr.commit_hash))
            .filter(|rr| !versions_in_proposals.contains(&rr.commit_hash))
            .collect())
    }

    async fn retireable_guestos_versions(&self) -> anyhow::Result<Vec<Release>> {
        let ic_repo = self.ic_repo();
        let guests = ic_repo.guestos_releases().await?;
        let active_releases = guests.get_active_branches();
        let subnet_versions: BTreeSet<String> = self.registry.subnets()?.values().map(|s| s.replica_version.clone()).collect();
        let version_on_unassigned_nodes = self.registry.unassigned_nodes_replica_version()?;
        let versions_in_proposals: BTreeSet<String> = self
            .proposal_agent
            .list_open_elect_replica_proposals()
            .await?
            .iter()
            .flat_map(|p| p.versions_unelect.iter())
            .cloned()
            .collect();
        info!("Active releases: {}", active_releases.iter().join(", "));
        info!("GuestOS versions in use on subnets: {}", subnet_versions.iter().join(", "));
        info!("GuestOS version on unassigned nodes: {}", version_on_unassigned_nodes);
        info!("GuestOS versions in open proposals: {}", versions_in_proposals.iter().join(", "));
        let guestos_releases = ic_repo.guestos_releases().await?;
        Ok(guestos_releases
            .releases
            .clone()
            .into_iter()
            .filter(|rr| !active_releases.contains(&rr.branch))
            .filter(|rr| !subnet_versions.contains(&rr.commit_hash) && rr.commit_hash != *version_on_unassigned_nodes)
            .filter(|rr| !versions_in_proposals.contains(&rr.commit_hash))
            .collect())
    }

    async fn run_membership_change(&self, change: SubnetChangeResponse, options: ProposeOptions) -> anyhow::Result<()> {
        let subnet_id = change.subnet_id.ok_or_else(|| anyhow::anyhow!("subnet_id is required"))?;
        let pending_action = self
            .registry
            .subnets_with_proposals()
            .await?
            .get(&subnet_id)
            .map(|s| s.proposal.clone())
            .ok_or(NetworkError::SubnetNotFound(subnet_id))?;

        if let Some(proposal) = pending_action {
            return Err(anyhow::anyhow!(format!(
                "There is a pending proposal for this subnet: https://dashboard.internetcomputer.org/proposal/{}",
                proposal.id
            )));
        }

        self.ic_admin
            .propose_run(
                ProposeCommand::ChangeSubnetMembership {
                    subnet_id,
                    node_ids_add: change.added.clone(),
                    node_ids_remove: change.removed.clone(),
                },
                options,
            )
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }
}

pub fn replace_proposal_options(change: &SubnetChangeResponse) -> anyhow::Result<ic_admin::ProposeOptions> {
    let subnet_id = change.subnet_id.ok_or_else(|| anyhow::anyhow!("subnet_id is required"))?.to_string();

    let replace_target = if change.added.len() > 1 || change.removed.len() > 1 {
        "nodes"
    } else {
        "a node"
    };
    let subnet_id_short = subnet_id.split('-').next().unwrap();

    Ok(ic_admin::ProposeOptions {
        title: format!("Replace {replace_target} in subnet {subnet_id_short}",).into(),
        summary: format!("# Replace {replace_target} in subnet {subnet_id_short}",).into(),
        motivation: change.motivation.clone(),
    })
}
