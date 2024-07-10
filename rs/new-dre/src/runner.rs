use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::Arc;

use decentralization::network::AvailableNodesQuerier;
use decentralization::network::NetworkHealRequest;
use decentralization::network::SubnetChange;
use decentralization::network::SubnetQuerier;
use decentralization::network::SubnetQueryBy;
use decentralization::network::TopologyManager;
use decentralization::subnets::NodesRemover;
use decentralization::SubnetChangeResponse;
use futures::TryFutureExt;
use futures_util::future::try_join;
use ic_management_backend::health;
use ic_management_backend::health::HealthStatusQuerier;
use ic_management_backend::lazy_git::LazyGit;
use ic_management_backend::lazy_registry::LazyRegistry;
use ic_management_backend::proposal::ProposalAgent;
use ic_management_backend::registry::ReleasesOps;
use ic_management_types::Artifact;
use ic_management_types::Network;
use ic_management_types::NetworkError;
use ic_management_types::Node;
use ic_management_types::NodeFeature;
use ic_management_types::Release;
use ic_management_types::TopologyChangePayload;
use ic_types::PrincipalId;
use itertools::Itertools;
use log::info;
use log::warn;

use registry_canister::mutations::do_change_subnet_membership::ChangeSubnetMembershipPayload;
use tabled::builder::Builder;
use tabled::settings::Style;

use crate::ic_admin::{self, IcAdminWrapper};
use crate::ic_admin::{ProposeCommand, ProposeOptions};
use crate::operations::hostos_rollout::HostosRollout;
use crate::operations::hostos_rollout::HostosRolloutResponse;
use crate::operations::hostos_rollout::NodeGroupUpdate;

pub struct Runner {
    ic_admin: Arc<IcAdminWrapper>,
    registry: Rc<LazyRegistry>,
    ic_repo: RefCell<Option<Rc<LazyGit>>>,
    network: Network,
    proposal_agent: ProposalAgent,
    verbose: bool,
}

impl Runner {
    pub fn new(ic_admin: Arc<IcAdminWrapper>, registry: Rc<LazyRegistry>, network: Network, agent: ProposalAgent, verbose: bool) -> Self {
        Self {
            ic_admin,
            registry,
            ic_repo: RefCell::new(None),
            network,
            proposal_agent: agent,
            verbose,
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

    pub async fn subnet_resize(&self, request: ic_management_types::requests::SubnetResizeRequest, motivation: String) -> anyhow::Result<()> {
        let change = self
            .registry
            .modify_subnet_nodes(SubnetQueryBy::SubnetId(request.subnet))
            .await?
            .excluding_from_available(request.exclude.clone().unwrap_or_default())
            .including_from_available(request.only.clone().unwrap_or_default())
            .including_from_available(request.include.clone().unwrap_or_default())
            .resize(request.add, request.remove)?;

        let change = SubnetChangeResponse::from(&change);

        if self.verbose {
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

        if self.verbose {
            if let Some(run_log) = &subnet_creation_data.run_log {
                println!("{}\n", run_log.join("\n"));
            }
        }
        println!("{}", subnet_creation_data);
        let replica_version = replica_version.unwrap_or(
            self.registry
                .nns_replica_version()
                .await
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

    pub async fn propose_subnet_change(&self, change: SubnetChangeResponse) -> anyhow::Result<()> {
        if self.verbose {
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

    pub async fn prepare_versions_to_retire(&self, release_artifact: &Artifact, edit_summary: bool) -> anyhow::Result<(String, Option<Vec<String>>)> {
        let retireable_versions = self.retireable_versions(release_artifact).await?;
        let versions = if retireable_versions.is_empty() {
            Vec::new()
        } else {
            info!("Waiting for you to pick the versions to retire in your editor");
            let template = "# In the below lines, comment out the versions that you DO NOT want to retire".to_string();
            let versions = edit::edit(format!(
                "{}\n{}",
                template,
                retireable_versions
                    .into_iter()
                    .map(|r| format!("{} # {}", r.commit_hash, r.branch))
                    .join("\n"),
            ))?
            .trim()
            .replace("\r(\n)?", "\n")
            .split('\n')
            .map(|s| regex::Regex::new("#.+$").unwrap().replace_all(s, "").to_string())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

            if versions.is_empty() {
                warn!("Empty list of GuestOS versions to unelect");
            }
            versions
        };

        let mut template =
            "Removing the obsolete GuestOS versions from the registry, to prevent unintended version downgrades in the future".to_string();
        if edit_summary {
            info!("Edit summary");
            template = edit::edit(template)?.trim().replace("\r(\n)?", "\n");
        }

        Ok((template, (!versions.is_empty()).then_some(versions)))
    }

    pub async fn do_revise_elected_replica_versions(
        &self,
        release_artifact: &Artifact,
        version: &String,
        release_tag: &String,
        force: bool,
    ) -> anyhow::Result<()> {
        let update_version = IcAdminWrapper::prepare_to_propose_to_revise_elected_versions(
            release_artifact,
            version,
            release_tag,
            force,
            self.prepare_versions_to_retire(release_artifact, false).await.map(|r| r.1)?,
        )
        .await?;

        self.ic_admin
            .propose_run(
                ProposeCommand::ReviseElectedVersions {
                    release_artifact: update_version.release_artifact.clone(),
                    args: update_version.get_update_cmd_args(),
                },
                ProposeOptions {
                    title: Some(update_version.title),
                    summary: Some(update_version.summary.clone()),
                    motivation: None,
                },
            )
            .await?;
        Ok(())
    }

    pub async fn hostos_rollout_nodes(
        &self,
        node_group: NodeGroupUpdate,
        version: &String,
        only: &[String],
        exclude: &[String],
    ) -> anyhow::Result<Option<(Vec<PrincipalId>, String)>> {
        let elected_versions = self.registry.elected_hostos().unwrap();
        if !elected_versions.contains(&version.to_string()) {
            return Err(anyhow::anyhow!(format!(
                "The version {} has not being elected.\nVersions elected are: {:?}",
                version, elected_versions,
            )));
        }

        let hostos_rollout = HostosRollout::new(
            self.registry.nodes().await?,
            self.registry.subnets().await?,
            &self.network,
            self.proposal_agent.clone(),
            version,
            only,
            exclude,
        );

        match hostos_rollout.execute(node_group).await? {
            HostosRolloutResponse::Ok(nodes_to_update, maybe_subnets_affected) => {
                let mut summary = "## List of nodes\n".to_string();
                let mut builder_dc = Builder::default();
                let nodes_by_dc = nodes_by_dc(nodes_to_update.clone());
                builder_dc.push_record(["dc", "node_id", "subnet"]);
                nodes_by_dc.into_iter().for_each(|(dc, nodes_with_sub)| {
                    builder_dc.push_record([
                        dc,
                        nodes_with_sub.iter().map(|(p, _)| p.to_string()).join("\n"),
                        nodes_with_sub.iter().map(|(_, s)| s.split('-').next().unwrap().to_string()).join("\n"),
                    ]);
                });

                let mut table_dc = builder_dc.build();
                table_dc.with(Style::markdown());
                summary.push_str(table_dc.to_string().as_str());
                summary.push_str("\n\n");

                if let Some(subnets_affected) = maybe_subnets_affected {
                    summary.push_str("## Updated nodes per subnet\n");
                    let mut builder_subnets = Builder::default();
                    builder_subnets.push_record(["subnet_id", "updated_nodes", "count", "subnet_size", "percent_subnet"]);

                    subnets_affected
                        .into_iter()
                        .map(|subnet| {
                            let nodes_id = nodes_to_update
                                .iter()
                                .cloned()
                                .filter_map(|n| {
                                    if n.subnet_id.unwrap_or_default() == subnet.subnet_id {
                                        Some(n.principal)
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<PrincipalId>>();
                            let subnet_id = subnet.subnet_id.to_string().split('-').next().unwrap().to_string();
                            let updated_nodes = nodes_id.iter().map(|p| p.to_string().split('-').next().unwrap().to_string()).join("\n");
                            let updates_nodes_count = nodes_id.len().to_string();
                            let subnet_size = subnet.subnet_size.to_string();
                            let percent_of_subnet_size = format!("{}%", (nodes_id.len() as f32 / subnet.subnet_size as f32 * 100.0).round());

                            [subnet_id, updated_nodes, updates_nodes_count, subnet_size.clone(), percent_of_subnet_size]
                        })
                        .sorted_by(|a, b| a[3].cmp(&b[3]))
                        .for_each(|row| {
                            builder_subnets.push_record(row);
                        });

                    let mut table_subnets = builder_subnets.build();
                    table_subnets.with(Style::markdown());
                    summary.push_str(table_subnets.to_string().as_str());
                };
                Ok(Some((nodes_to_update.into_iter().map(|n| n.principal).collect::<Vec<_>>(), summary)))
            }
            HostosRolloutResponse::None(reason) => {
                reason
                    .iter()
                    .for_each(|(group, reason)| println!("No nodes to update in group: {} because: {}", group, reason));
                Ok(None)
            }
        }
    }

    pub async fn hostos_rollout(&self, nodes: Vec<PrincipalId>, version: &str, maybe_summary: Option<String>) -> anyhow::Result<()> {
        let title = format!("Set HostOS version: {version} on {} nodes", nodes.clone().len());

        self.ic_admin
            .propose_run(
                ProposeCommand::DeployHostosToSomeNodes {
                    nodes: nodes.clone(),
                    version: version.to_string(),
                },
                ProposeOptions {
                    title: title.clone().into(),
                    summary: maybe_summary.unwrap_or(title).into(),
                    motivation: None,
                },
            )
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        let nodes_short = nodes
            .iter()
            .map(|p| p.to_string().split('-').next().unwrap().to_string())
            .collect::<Vec<_>>();
        println!("Submitted proposal to update the following nodes: {:?}", nodes_short);
        println!("You can follow the upgrade progress at https://grafana.mainnet.dfinity.network/explore?orgId=1&left=%7B%22datasource%22:%22PE62C54679EC3C073%22,%22queries%22:%5B%7B%22refId%22:%22A%22,%22datasource%22:%7B%22type%22:%22prometheus%22,%22uid%22:%22PE62C54679EC3C073%22%7D,%22editorMode%22:%22code%22,%22expr%22:%22hostos_version%7Bic_node%3D~%5C%22{}%5C%22%7D%5Cn%22,%22legendFormat%22:%22__auto%22,%22range%22:true,%22instant%22:true%7D%5D,%22range%22:%7B%22from%22:%22now-1h%22,%22to%22:%22now%22%7D%7D", nodes_short.iter().map(|n| n.to_string() + ".%2B").join("%7C"));

        Ok(())
    }

    pub async fn remove_nodes(&self, nodes_remover: NodesRemover) -> anyhow::Result<()> {
        let health_client = health::HealthClient::new(self.network.clone());
        let (healths, nodes_with_proposals) = try_join(health_client.nodes(), self.registry.nodes_with_proposals()).await?;
        let (mut node_removals, motivation) = nodes_remover.remove_nodes(healths, nodes_with_proposals);
        node_removals.sort_by_key(|nr| nr.reason.message());

        let headers = vec!["Principal".to_string()]
            .into_iter()
            .chain(NodeFeature::variants().iter().map(|nf| nf.to_string()))
            .chain(vec!["Hostname".to_string()].into_iter())
            .chain(vec!["Reason".to_string()].into_iter())
            .collect::<Vec<_>>();
        let mut table = tabular::Table::new(&headers.iter().map(|_| "    {:<}").collect::<Vec<_>>().join(""));
        // Headers
        let mut header_row = tabular::Row::new();
        for h in headers {
            header_row.add_cell(h);
        }
        table.add_row(header_row);

        // Values
        for nr in &node_removals {
            let mut row = tabular::Row::new();
            let decentralization_node = decentralization::network::Node::from(&nr.node);
            row.add_cell(nr.node.principal);
            for nf in NodeFeature::variants() {
                row.add_cell(decentralization_node.get_feature(&nf));
            }
            row.add_cell(nr.node.hostname.clone().unwrap_or_else(|| "N/A".to_string()));
            row.add_cell(nr.reason.message());
            table.add_row(row);
        }
        println!("{}", table);

        self.ic_admin
            .propose_run(
                ic_admin::ProposeCommand::RemoveNodes {
                    nodes: node_removals.iter().map(|n| n.node.principal).collect(),
                },
                ProposeOptions {
                    title: "Remove nodes from the network".to_string().into(),
                    summary: "Remove nodes from the network".to_string().into(),
                    motivation: motivation.into(),
                },
            )
            .await?;
        Ok(())
    }

    pub async fn network_heal(&self) -> anyhow::Result<()> {
        let health_client = health::HealthClient::new(self.network.clone());
        let mut errors = vec![];

        let subnets = self.registry.subnets().await?;
        let (available_nodes, healths) = try_join(self.registry.available_nodes().map_err(anyhow::Error::from), health_client.nodes()).await?;

        let subnets_change_response = NetworkHealRequest::new(subnets).heal_and_optimize(available_nodes, healths).await?;
        subnets_change_response.iter().for_each(|change| println!("{}", change));

        for change in &subnets_change_response {
            let _ = self
                .run_membership_change(change.clone(), replace_proposal_options(change)?)
                .await
                .map_err(|e| {
                    println!("{}", e);
                    errors.push(e);
                });
        }
        if !errors.is_empty() {
            anyhow::bail!("Errors: {:?}", errors);
        }

        Ok(())
    }

    pub async fn decentralization_change(&self, change: &ChangeSubnetMembershipPayload) -> anyhow::Result<()> {
        if let Some(id) = change.get_subnet() {
            let subnet_before = self.registry.subnet(SubnetQueryBy::SubnetId(id)).await.map_err(|e| anyhow::anyhow!(e))?;
            let nodes_before = subnet_before.nodes.clone();

            let added_nodes = self.registry.get_decentralized_nodes(&change.get_added_node_ids()).await?;
            let removed_nodes = self.registry.get_decentralized_nodes(&change.get_added_node_ids()).await?;

            let subnet_after = subnet_before
                .with_nodes(added_nodes)
                .without_nodes(removed_nodes)
                .map_err(|e| anyhow::anyhow!(e))?;

            let subnet_change = SubnetChange {
                id: subnet_after.id,
                old_nodes: nodes_before,
                new_nodes: subnet_after.nodes,
                ..Default::default()
            };
            println!("{}", SubnetChangeResponse::from(&subnet_change))
        }
        Ok(())
    }

    pub async fn subnet_rescue(&self, subnet: &PrincipalId, keep_nodes: Option<Vec<String>>) -> anyhow::Result<()> {
        let change_request = self
            .registry
            .modify_subnet_nodes(SubnetQueryBy::SubnetId(*subnet))
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        let change_request = match keep_nodes {
            Some(n) => change_request.keeping_from_used(n),
            None => change_request,
        };

        let change = SubnetChangeResponse::from(&change_request.rescue()?);
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
        let hostos_versions: BTreeSet<String> = self.registry.nodes().await?.values().map(|s| s.hostos_version.clone()).collect();
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
        let subnet_versions: BTreeSet<String> = self.registry.subnets().await?.values().map(|s| s.replica_version.clone()).collect();
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

fn nodes_by_dc(nodes: Vec<Node>) -> BTreeMap<String, Vec<(String, String)>> {
    nodes
        .iter()
        .cloned()
        .map(|n| {
            let subnet_name = if n.subnet_id.is_some() {
                n.subnet_id.unwrap_or_default().0.to_string()
            } else {
                String::from("<unassigned>")
            };
            (
                n.principal.to_string().split('-').next().unwrap().to_string(),
                subnet_name,
                n.operator.datacenter,
            )
        })
        .fold(BTreeMap::new(), |mut acc, (node_id, subnet, dc)| {
            acc.entry(dc.unwrap_or_default().name).or_default().push((node_id, subnet));
            acc
        })
}
