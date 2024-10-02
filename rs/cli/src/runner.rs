use std::cell::RefCell;
use std::sync::Arc;

use decentralization::network::DecentralizedSubnet;
use decentralization::network::NetworkHealRequest;
use decentralization::network::SubnetChange;
use decentralization::network::SubnetQueryBy;
use decentralization::network::{generate_added_node_description, generate_removed_nodes_description};
use decentralization::subnets::NodesRemover;
use decentralization::SubnetChangeResponse;
use futures::TryFutureExt;
use futures_util::future::try_join;
use ic_management_backend::health;
use ic_management_backend::health::HealthStatusQuerier;
use ic_management_backend::lazy_git::LazyGit;
use ic_management_backend::lazy_git::LazyGitImpl;
use ic_management_backend::lazy_registry::LazyRegistry;
use ic_management_backend::proposal::ProposalAgent;
use ic_management_backend::registry::ReleasesOps;
use ic_management_types::Artifact;
use ic_management_types::HealthStatus;
use ic_management_types::Network;
use ic_management_types::NetworkError;
use ic_management_types::Node;
use ic_management_types::NodeFeature;
use ic_management_types::Release;
use ic_management_types::TopologyChangePayload;
use ic_types::PrincipalId;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use log::info;
use log::warn;

use regex::Regex;
use registry_canister::mutations::do_change_subnet_membership::ChangeSubnetMembershipPayload;
use tabled::builder::Builder;
use tabled::settings::Style;

use crate::artifact_downloader::ArtifactDownloader;
use crate::ic_admin;
use crate::ic_admin::{ProposeCommand, ProposeOptions};
use crate::operations::hostos_rollout::HostosRollout;
use crate::operations::hostos_rollout::HostosRolloutResponse;
use crate::operations::hostos_rollout::NodeGroupUpdate;

pub struct Runner {
    registry: Arc<dyn LazyRegistry>,
    ic_repo: RefCell<Option<Arc<dyn LazyGit>>>,
    network: Network,
    proposal_agent: Arc<dyn ProposalAgent>,
    verbose: bool,
    artifact_downloader: Arc<dyn ArtifactDownloader>,
}

#[derive(Clone)]
pub struct RunnerProposal {
    pub cmd: ProposeCommand,
    pub opts: ProposeOptions,
}

impl Runner {
    pub fn new(
        registry: Arc<dyn LazyRegistry>,
        network: Network,
        agent: Arc<dyn ProposalAgent>,
        verbose: bool,
        ic_repo: RefCell<Option<Arc<dyn LazyGit>>>,
        artifact_downloader: Arc<dyn ArtifactDownloader>,
    ) -> Self {
        Self {
            registry,
            ic_repo,
            network,
            proposal_agent: agent,
            verbose,
            artifact_downloader,
        }
    }

    async fn ic_repo(&self) -> Arc<dyn LazyGit> {
        if let Some(ic_repo) = self.ic_repo.borrow().as_ref() {
            return ic_repo.clone();
        }

        let ic_repo = Arc::new(
            LazyGitImpl::new(
                self.network.clone(),
                self.registry
                    .elected_guestos()
                    .await
                    .expect("Should be able to fetch elected guestos versions")
                    .to_vec(),
                self.registry
                    .elected_hostos()
                    .await
                    .expect("Should be able to fetch elected hostos versions")
                    .to_vec(),
            )
            .expect("Should be able to create IC repo"),
        ) as Arc<dyn LazyGit>;
        *self.ic_repo.borrow_mut() = Some(ic_repo.clone());
        ic_repo
    }

    pub async fn deploy(&self, subnet: &PrincipalId, version: &str, forum_post_link: Option<String>) -> anyhow::Result<RunnerProposal> {
        Ok(RunnerProposal {
            cmd: ProposeCommand::DeployGuestosToAllSubnetNodes {
                subnet: *subnet,
                version: version.to_owned(),
            },
            opts: ProposeOptions {
                title: format!("Update subnet {subnet} to GuestOS version {version}").into(),
                summary: format!("Update subnet {subnet} to GuestOS version {version}").into(),
                motivation: None,
                forum_post_link,
            },
        })
    }

    pub async fn health_of_nodes(&self) -> anyhow::Result<IndexMap<PrincipalId, HealthStatus>> {
        let health_client = health::HealthClient::new(self.network.clone());
        health_client.nodes().await
    }

    pub async fn subnet_create(
        &self,
        request: ic_management_types::requests::SubnetCreateRequest,
        motivation: String,
        forum_post_link: Option<String>,
        replica_version: Option<String>,
        other_args: Vec<String>,
    ) -> anyhow::Result<Option<RunnerProposal>> {
        let health_of_nodes = self.health_of_nodes().await?;

        let subnet_creation_data = self
            .registry
            .create_subnet(
                request.size,
                request.include.clone().unwrap_or_default(),
                request.exclude.clone().unwrap_or_default(),
                request.only.clone().unwrap_or_default(),
                &health_of_nodes,
            )
            .await?;
        let subnet_creation_data = SubnetChangeResponse::from(&subnet_creation_data).with_health_of_nodes(health_of_nodes.clone());

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

        Ok(Some(RunnerProposal {
            cmd: ProposeCommand::CreateSubnet {
                node_ids: subnet_creation_data.added_with_desc.iter().map(|a| a.0).collect::<Vec<_>>(),
                replica_version,
                other_args,
            },
            opts: ProposeOptions {
                title: Some("Creating new subnet".into()),
                summary: Some("# Creating new subnet with nodes: ".into()),
                motivation: Some(motivation.clone()),
                forum_post_link,
            },
        }))
    }

    pub async fn propose_subnet_change(
        &self,
        change: SubnetChangeResponse,
        forum_post_link: Option<String>,
    ) -> anyhow::Result<Option<RunnerProposal>> {
        if self.verbose {
            if let Some(run_log) = &change.run_log {
                println!("{}\n", run_log.join("\n"));
            }
        }

        if change.added_with_desc.is_empty() && change.removed_with_desc.is_empty() {
            return Ok(None);
        }

        let options = replace_proposal_options(&change, forum_post_link)?;
        self.run_membership_change(change, options).await.map(|prop| Some(prop))
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
        version: &str,
        release_tag: &str,
        ignore_missing_urls: bool,
        forum_post_link: String,
        security_fix: bool,
    ) -> anyhow::Result<RunnerProposal> {
        let update_version = self
            .prepare_to_propose_to_revise_elected_versions(
                release_artifact,
                version,
                release_tag,
                ignore_missing_urls,
                self.prepare_versions_to_retire(release_artifact, false).await.map(|r| r.1)?,
                security_fix,
                forum_post_link.clone(),
            )
            .await?;

        Ok(RunnerProposal {
            cmd: ProposeCommand::ReviseElectedVersions {
                release_artifact: update_version.release_artifact.clone(),
                args: update_version.get_update_cmd_args(),
            },
            opts: ProposeOptions {
                title: Some(update_version.title),
                summary: Some(update_version.summary.clone()),
                motivation: None,
                forum_post_link: Some(forum_post_link),
            },
        })
    }

    async fn prepare_to_propose_to_revise_elected_versions(
        &self,
        release_artifact: &Artifact,
        version: &str,
        release_tag: &str,
        ignore_missing_urls: bool,
        retire_versions: Option<Vec<String>>,
        security_fix: bool,
        forum_post_link: String,
    ) -> anyhow::Result<UpdateVersion> {
        let (update_urls, expected_hash) = self
            .artifact_downloader
            .download_images_and_validate_sha256(release_artifact, version, ignore_missing_urls)
            .await?;

        let summary = match security_fix {
            true => format_security_hotfix(forum_post_link),
            false => format_regular_version_upgrade_summary(version, release_artifact, release_tag, forum_post_link)?,
        };
        if summary.contains("Remove this block of text from the proposal.") {
            Err(anyhow::anyhow!("The edited proposal text has not been edited to add release notes."))
        } else {
            let proposal_title = match security_fix {
                true => "Security patch update".to_string(),
                false => match &retire_versions {
                    Some(v) => {
                        let pluralize = if v.len() == 1 { "version" } else { "versions" };
                        format!(
                            "Elect new IC/{} revision (commit {}), and retire old replica {} {}",
                            release_artifact.capitalized(),
                            &version[..8],
                            pluralize,
                            v.iter().map(|v| &v[..8]).join(",")
                        )
                    }
                    None => format!("Elect new IC/{} revision (commit {})", release_artifact.capitalized(), &version[..8]),
                },
            };

            Ok(UpdateVersion {
                release_artifact: release_artifact.clone(),
                version: version.to_string(),
                title: proposal_title.clone(),
                stringified_hash: expected_hash,
                summary,
                update_urls,
                versions_to_retire: retire_versions.clone(),
            })
        }
    }

    pub async fn hostos_rollout_nodes(
        &self,
        node_group: NodeGroupUpdate,
        version: &String,
        only: &[String],
        exclude: &[String],
    ) -> anyhow::Result<Option<(Vec<PrincipalId>, String)>> {
        let elected_versions = self.registry.elected_hostos().await.unwrap();
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

    pub fn hostos_rollout(
        &self,
        nodes: Vec<PrincipalId>,
        version: &str,
        maybe_summary: Option<String>,
        forum_post_link: Option<String>,
    ) -> anyhow::Result<RunnerProposal> {
        let title = format!("Set HostOS version: {version} on {} nodes", nodes.clone().len());

        let nodes_short = nodes
            .iter()
            .map(|p| p.to_string().split('-').next().unwrap().to_string())
            .collect::<Vec<_>>();
        println!("Will submit proposal to update the following nodes: {:?}", nodes_short);
        println!("You will be able to follow the upgrade progress at https://grafana.mainnet.dfinity.network/explore?orgId=1&left=%7B%22datasource%22:%22PE62C54679EC3C073%22,%22queries%22:%5B%7B%22refId%22:%22A%22,%22datasource%22:%7B%22type%22:%22prometheus%22,%22uid%22:%22PE62C54679EC3C073%22%7D,%22editorMode%22:%22code%22,%22expr%22:%22hostos_version%7Bic_node%3D~%5C%22{}%5C%22%7D%5Cn%22,%22legendFormat%22:%22__auto%22,%22range%22:true,%22instant%22:true%7D%5D,%22range%22:%7B%22from%22:%22now-1h%22,%22to%22:%22now%22%7D%7D", nodes_short.iter().map(|n| n.to_string() + ".%2B").join("%7C"));

        Ok(RunnerProposal {
            cmd: ProposeCommand::DeployHostosToSomeNodes {
                nodes: nodes.clone(),
                version: version.to_string(),
            },
            opts: ProposeOptions {
                title: title.clone().into(),
                summary: maybe_summary.unwrap_or(title).into(),
                motivation: None,
                forum_post_link,
            },
        })
    }

    pub async fn remove_nodes(&self, nodes_remover: NodesRemover) -> anyhow::Result<RunnerProposal> {
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

        if node_removals.is_empty() {
            anyhow::bail!("Calculated 0 node removals");
        }
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

        Ok(RunnerProposal {
            cmd: ic_admin::ProposeCommand::RemoveNodes {
                nodes: node_removals.iter().map(|n| n.node.principal).collect(),
            },
            opts: ProposeOptions {
                title: "Remove nodes from the network".to_string().into(),
                summary: "Remove nodes from the network".to_string().into(),
                motivation: motivation.into(),
                forum_post_link: nodes_remover.forum_post_link,
            },
        })
    }

    pub async fn network_heal(&self, forum_post_link: Option<String>) -> anyhow::Result<Vec<RunnerProposal>> {
        let health_client = health::HealthClient::new(self.network.clone());
        let mut errors = vec![];

        // Get the list of subnets, and the list of open proposal for each subnet, if any
        let subnets = self.registry.subnets_and_proposals().await?;
        let subnets_without_proposals = subnets
            .iter()
            .filter(|(subnet_id, subnet)| match &subnet.proposal {
                Some(p) => {
                    info!("Skipping subnet {} as it has a pending proposal {}", subnet_id, p.id);
                    false
                }
                None => true,
            })
            .map(|(id, subnet)| (*id, subnet.clone()))
            .collect::<IndexMap<_, _>>();
        let (available_nodes, health_of_nodes) =
            try_join(self.registry.available_nodes().map_err(anyhow::Error::from), health_client.nodes()).await?;

        let subnets_change_response = NetworkHealRequest::new(subnets_without_proposals)
            .heal_and_optimize(available_nodes, &health_of_nodes)
            .await?;

        let mut changes = vec![];
        for change in &subnets_change_response {
            let current = self
                .run_membership_change(change.clone(), replace_proposal_options(change, forum_post_link.clone())?)
                .await
                .map_err(|e| {
                    println!("{}", e);
                    errors.push(e);
                });
            changes.push(current)
        }
        if !errors.is_empty() {
            anyhow::bail!("Errors: {:?}", errors);
        }

        // No errors, can be safly unwrapped
        Ok(changes.into_iter().map(|maybe_change| maybe_change.unwrap()).collect_vec())
    }

    pub async fn decentralization_change(
        &self,
        change: &ChangeSubnetMembershipPayload,
        override_subnet_nodes: Option<Vec<PrincipalId>>,
    ) -> anyhow::Result<()> {
        let subnet_before = match override_subnet_nodes {
            Some(nodes) => {
                let nodes = self.registry.get_decentralized_nodes(&nodes).await?;
                DecentralizedSubnet::new_with_subnet_id_and_nodes(change.subnet_id, nodes)
            }
            None => self
                .registry
                .subnet(SubnetQueryBy::SubnetId(change.subnet_id))
                .await
                .map_err(|e| anyhow::anyhow!(e))?,
        };
        let nodes_before = subnet_before.nodes.clone();
        let health_of_nodes = self.health_of_nodes().await?;

        // Simulate node removal
        let removed_nodes = self.registry.get_decentralized_nodes(&change.get_removed_node_ids()).await?;
        let removed_nodes_with_desc = generate_removed_nodes_description(&subnet_before.nodes, &removed_nodes);
        let subnet_mid = subnet_before
            .without_nodes(removed_nodes_with_desc.clone())
            .map_err(|e| anyhow::anyhow!(e))?;

        // Now simulate node addition
        let added_nodes = self.registry.get_decentralized_nodes(&change.get_added_node_ids()).await?;
        let added_nodes_with_desc = generate_added_node_description(&subnet_mid.nodes, &added_nodes);

        let subnet_after = subnet_mid.with_nodes(added_nodes_with_desc.clone());

        let subnet_change = SubnetChange {
            id: subnet_after.id,
            old_nodes: nodes_before,
            new_nodes: subnet_after.nodes,
            added_nodes_desc: added_nodes_with_desc.clone(),
            removed_nodes_desc: removed_nodes_with_desc.clone(),
            ..Default::default()
        };
        println!(
            "{}",
            SubnetChangeResponse::from(&subnet_change).with_health_of_nodes(health_of_nodes.clone())
        );
        Ok(())
    }

    pub async fn subnet_rescue(
        &self,
        subnet: &PrincipalId,
        keep_nodes: Option<Vec<String>>,
        forum_post_link: Option<String>,
    ) -> anyhow::Result<Option<RunnerProposal>> {
        let change_request = self
            .registry
            .modify_subnet_nodes(SubnetQueryBy::SubnetId(*subnet))
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        let change_request = match keep_nodes {
            Some(n) => change_request.keeping_from_used(n),
            None => change_request,
        };

        let health_of_nodes = self.health_of_nodes().await?;

        let change = SubnetChangeResponse::from(&change_request.rescue(&health_of_nodes)?).with_health_of_nodes(health_of_nodes);

        if change.added_with_desc.is_empty() && change.removed_with_desc.is_empty() {
            return Ok(None);
        }

        self.run_membership_change(change.clone(), replace_proposal_options(&change, forum_post_link)?)
            .await
            .map(|prop| Some(prop))
    }

    pub async fn retireable_versions(&self, artifact: &Artifact) -> anyhow::Result<Vec<Release>> {
        match artifact {
            Artifact::GuestOs => self.retireable_guestos_versions().await,
            Artifact::HostOs => self.retireable_hostos_versions().await,
        }
    }

    async fn retireable_hostos_versions(&self) -> anyhow::Result<Vec<Release>> {
        let ic_repo = self.ic_repo().await;
        let hosts = ic_repo.hostos_releases().await?;
        let active_releases = hosts.get_active_branches();
        let hostos_versions: IndexSet<String> = self.registry.nodes().await?.values().map(|s| s.hostos_version.clone()).collect();
        let versions_in_proposals: IndexSet<String> = self
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
        let ic_repo = self.ic_repo().await;
        let guests = ic_repo.guestos_releases().await?;
        let active_releases = guests.get_active_branches();
        let subnet_versions: IndexSet<String> = self.registry.subnets().await?.values().map(|s| s.replica_version.clone()).collect();
        let version_on_unassigned_nodes = self.registry.unassigned_nodes_replica_version().await?;
        let versions_in_proposals: IndexSet<String> = self
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

    async fn run_membership_change(&self, change: SubnetChangeResponse, options: ProposeOptions) -> anyhow::Result<RunnerProposal> {
        let subnet_id = change.subnet_id.ok_or_else(|| anyhow::anyhow!("subnet_id is required"))?;
        let pending_action = self
            .registry
            .subnets_and_proposals()
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

        Ok(RunnerProposal {
            cmd: ProposeCommand::ChangeSubnetMembership {
                subnet_id,
                node_ids_add: change.added_with_desc.iter().map(|a| a.0).collect::<Vec<_>>(),
                node_ids_remove: change.removed_with_desc.iter().map(|a| a.0).collect::<Vec<_>>(),
            },
            opts: options,
        })
    }

    pub async fn update_unassigned_nodes(
        &self,
        nns_subnet_id: &PrincipalId,
        forum_post_link: Option<String>,
    ) -> anyhow::Result<Option<RunnerProposal>> {
        let subnets = self.registry.subnets().await?;

        let nns = match subnets.get_key_value(nns_subnet_id) {
            Some((_, value)) => value,
            None => return Err(anyhow::anyhow!("Couldn't find nns subnet with id '{}'", nns_subnet_id)),
        };

        let unassigned_version = self.registry.unassigned_nodes_replica_version().await?;

        if unassigned_version == nns.replica_version.clone().into() {
            info!(
                "Unassigned nodes and nns are of the same version '{}', skipping proposal submition.",
                unassigned_version
            );
            return Ok(None);
        }

        info!(
            "NNS version '{}' and Unassigned nodes '{}' differ",
            nns.replica_version, unassigned_version
        );

        Ok(Some(RunnerProposal {
            cmd: ProposeCommand::DeployGuestosToAllUnassignedNodes {
                replica_version: nns.replica_version.clone(),
            },
            opts: ProposeOptions {
                summary: Some("Update the unassigned nodes to the latest rolled-out version".to_string()),
                motivation: None,
                title: Some("Update all unassigned nodes".to_string()),
                forum_post_link,
            },
        }))
    }
}

pub fn replace_proposal_options(change: &SubnetChangeResponse, forum_post_link: Option<String>) -> anyhow::Result<ic_admin::ProposeOptions> {
    let subnet_id = change.subnet_id.ok_or_else(|| anyhow::anyhow!("subnet_id is required"))?.to_string();

    let replace_target = if change.added_with_desc.len() > 1 || change.removed_with_desc.len() > 1 {
        "nodes"
    } else {
        "a node"
    };
    let subnet_id_short = subnet_id.split('-').next().unwrap();

    let change_desc = if change.added_with_desc.len() == change.removed_with_desc.len() {
        format!("Replace {} in subnet {}", replace_target, subnet_id_short)
    } else {
        format!("Resize subnet {}", subnet_id_short)
    };

    Ok(ic_admin::ProposeOptions {
        title: Some(change_desc.clone()),
        summary: Some(format!("# {change_desc}")),
        motivation: Some(format!("{}\n\n{}\n", change.motivation.as_ref().unwrap_or(&String::new()), change)),
        forum_post_link,
    })
}

fn nodes_by_dc(nodes: Vec<Node>) -> IndexMap<String, Vec<(String, String)>> {
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
        .fold(IndexMap::new(), |mut acc, (node_id, subnet, dc)| {
            acc.entry(dc.unwrap_or_default().name).or_default().push((node_id, subnet));
            acc
        })
}

#[derive(Clone)]
pub struct UpdateVersion {
    pub release_artifact: Artifact,
    pub version: String,
    pub title: String,
    pub summary: String,
    pub update_urls: Vec<String>,
    pub stringified_hash: String,
    pub versions_to_retire: Option<Vec<String>>,
}

impl UpdateVersion {
    pub fn get_update_cmd_args(&self) -> Vec<String> {
        [
            [
                vec![
                    "--replica-version-to-elect".to_string(),
                    self.version.to_string(),
                    "--release-package-sha256-hex".to_string(),
                    self.stringified_hash.to_string(),
                    "--release-package-urls".to_string(),
                ],
                self.update_urls.clone(),
            ]
            .concat(),
            match self.versions_to_retire.clone() {
                Some(versions) => [vec!["--replica-versions-to-unelect".to_string()], versions].concat(),
                None => vec![],
            },
        ]
        .concat()
    }
}

pub fn format_regular_version_upgrade_summary(
    version: &str,
    release_artifact: &Artifact,
    release_tag: &str,
    forum_post_link: String,
) -> anyhow::Result<String> {
    let template = format!(
        r#"Elect new {release_artifact} binary revision [{version}](https://github.com/dfinity/ic/tree/{release_tag})

    # Release Notes:

    [comment]: <> Remove this block of text from the proposal.
    [comment]: <> Then, add the {release_artifact} binary release notes as bullet points here.
    [comment]: <> Any [commit ID] within square brackets will auto-link to the specific changeset.

    # IC-OS Verification

    To build and verify the IC-OS disk image, run:

    ```
    # From https://github.com/dfinity/ic#verifying-releases
    sudo apt-get install -y curl && curl --proto '=https' --tlsv1.2 -sSLO https://raw.githubusercontent.com/dfinity/ic/{version}/ci/tools/repro-check.sh && chmod +x repro-check.sh && ./repro-check.sh -c {version}
    ```

    The two SHA256 sums printed above from a) the downloaded CDN image and b) the locally built image,
    must be identical, and must match the SHA256 from the payload of the NNS proposal.

    Forum post link: {forum_post_link}
    "#
    );

    // Remove <!--...--> from the commit
    // Leading or trailing spaces are removed as well and replaced with a single space.
    // Regex can be analyzed and tested at:
    // https://rregex.dev/?version=1.7&method=replace&regex=%5Cs*%3C%21--.%2B%3F--%3E%5Cs*&replace=+&text=*+%5Babc%5D+%3C%21--+ignored+1+--%3E+line%0A*+%5Babc%5D+%3C%21--+ignored+2+--%3E+comment+1+%3C%21--+ignored+3+--%3E+comment+2%0A
    let re_comment = Regex::new(r"\s*<!--.+?-->\s*").unwrap();

    Ok(match cfg!(test) {
        true => template.lines().map(|l| l.trim()).filter(|l| !l.starts_with("[comment]")).join("\n"),
        false => {
            let mut builder = edit::Builder::new();
            let with_suffix = builder.suffix(".md");
            edit::edit_with_builder(template, with_suffix)?
        }
    }
    .trim()
    .replace("\r(\n)?", "\n")
    .split('\n')
    .map(|f| {
        let f = re_comment.replace_all(f.trim(), " ");

        if !f.starts_with('*') {
            return f.to_string();
        }
        match f.split_once(']') {
            Some((left, message)) => {
                let commit_hash = left.split_once('[').unwrap().1.to_string();

                format!("* [[{}](https://github.com/dfinity/ic/commit/{})] {}", commit_hash, commit_hash, message)
            }
            None => f.to_string(),
        }
    })
    .join("\n"))
}

pub fn format_security_hotfix(forum_post_link: String) -> String {
    format!(r#"In accordance with the Security Patch Policy and Procedure that was adopted in proposal [48792](https://dashboard.internetcomputer.org/proposal/48792), the source code that was used to build this release will be exposed at the latest 10 days after the fix is rolled out to all subnets.

    The community will be able to retroactively verify the binaries that were rolled out.

    Forum post link: {forum_post_link}
"#).lines().map(|l| l.trim()).join("\n")
}
