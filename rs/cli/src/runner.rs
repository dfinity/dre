use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use ahash::AHashMap;
use decentralization::network::DecentralizedSubnet;
use decentralization::network::NetworkHealRequest;
use decentralization::network::NodeFeaturePair;
use decentralization::network::SubnetChange;
use decentralization::network::SubnetChangeRequest;
use decentralization::network::SubnetQueryBy;
use decentralization::subnets::NodesRemover;
use decentralization::SubnetChangeResponse;
use futures::TryFutureExt;
use futures_util::future::try_join;
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
use ic_management_types::Operator;
use ic_management_types::Release;
use ic_management_types::Subnet;
use ic_management_types::TopologyChangePayload;
use ic_types::PrincipalId;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use log::info;
use log::warn;

use regex::Regex;
use registry_canister::mutations::do_change_subnet_membership::ChangeSubnetMembershipPayload;
use reqwest::get;
use tabled::builder::Builder;
use tabled::settings::Style;

use crate::artifact_downloader::ArtifactDownloader;
use crate::cordoned_feature_fetcher::CordonedFeatureFetcher;
use crate::ic_admin::{self};
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
    cordoned_features_fetcher: Arc<dyn CordonedFeatureFetcher>,
    health_client: Arc<dyn HealthStatusQuerier>,
}

#[derive(Clone, Debug)]
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
        cordoned_features_fetcher: Arc<dyn CordonedFeatureFetcher>,
        health_client: Arc<dyn HealthStatusQuerier>,
    ) -> Self {
        Self {
            registry,
            ic_repo,
            network,
            proposal_agent: agent,
            verbose,
            artifact_downloader,
            cordoned_features_fetcher,
            health_client,
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
        self.health_client.nodes().await
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
                self.cordoned_features_fetcher.fetch().await.unwrap_or_else(|e| {
                    warn!("Failed to fetch cordoned features with error: {:?}", e);
                    warn!("Will continue running as if no features were cordoned");
                    vec![]
                }),
            )
            .await?;
        let subnet_creation_data = SubnetChangeResponse::new(&subnet_creation_data, &health_of_nodes, Some(motivation.clone()));

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
                node_ids: subnet_creation_data.node_ids_added,
                replica_version,
                other_args,
            },
            opts: ProposeOptions {
                title: Some("Creating new subnet".into()),
                summary: Some("# Creating new subnet with nodes: ".into()),
                motivation: Some(motivation),
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

        if change.node_ids_added.is_empty() && change.node_ids_removed.is_empty() {
            return Ok(None);
        }

        let options = replace_proposal_options(&change, forum_post_link).await?;
        self.run_membership_change(change, options).await.map(Some)
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
            self.proposal_agent.clone(),
            version,
            only,
            exclude,
            self.health_client.clone(),
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
        let (healths, nodes_with_proposals) = try_join(self.health_client.nodes(), self.registry.nodes_and_proposals()).await?;
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

    pub async fn network_heal(&self, forum_post_link: Option<String>, skip_subnets: &[String]) -> anyhow::Result<Vec<RunnerProposal>> {
        let mut errors = vec![];

        // Get the list of subnets, and the list of open proposal for each subnet, if any
        let subnets = self.registry.subnets_and_proposals().await?;
        let subnets_not_skipped = subnets
            .iter()
            .filter(|(subnet_id, _)| !skip_subnets.iter().any(|s| subnet_id.to_string().contains(s)))
            .map(|(subnet_id, subnet)| (*subnet_id, subnet.clone()))
            .collect::<IndexMap<_, _>>();
        let subnets_without_proposals = subnets_not_skipped
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
            try_join(self.registry.available_nodes().map_err(anyhow::Error::from), self.health_client.nodes()).await?;

        let subnets_change_responses = NetworkHealRequest::new(subnets_without_proposals)
            .heal_and_optimize(
                available_nodes,
                &health_of_nodes,
                self.cordoned_features_fetcher.fetch().await.unwrap_or_else(|e| {
                    warn!("Failed to fetch cordoned features with error: {:?}", e);
                    warn!("Will continue running as if no features were cordoned");
                    vec![]
                }),
            )
            .await?;

        let mut changes = vec![];
        for change in &subnets_change_responses {
            let current = self
                .run_membership_change(change.clone(), replace_proposal_options(change, forum_post_link.clone()).await?)
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

        // No errors, can be safely unwrapped
        Ok(changes.into_iter().map(|maybe_change| maybe_change.unwrap()).collect_vec())
    }

    async fn get_subnets(&self, skip_subnets: &[String]) -> anyhow::Result<IndexMap<PrincipalId, Subnet>> {
        let subnets = self.registry.subnets_and_proposals().await?;
        Ok(subnets
            .iter()
            .filter(|(subnet_id, _)| !skip_subnets.iter().any(|s| subnet_id.to_string().contains(s)))
            .map(|(subnet_id, subnet)| (*subnet_id, subnet.clone()))
            .collect::<IndexMap<_, _>>())
    }

    async fn get_available_and_healthy_nodes(&self) -> anyhow::Result<(Vec<decentralization::network::Node>, IndexMap<PrincipalId, HealthStatus>)> {
        try_join(self.registry.available_nodes().map_err(anyhow::Error::from), self.health_client.nodes()).await
    }

    fn get_operators_to_optimize(
        &self,
        node_operators_all: &IndexMap<PrincipalId, Operator>,
        all_nodes_grouped_by_operator: &HashMap<PrincipalId, Vec<decentralization::network::Node>>,
        available_nodes_grouped_by_operator: &HashMap<PrincipalId, Vec<decentralization::network::Node>>,
        nodes_all: &IndexMap<PrincipalId, Node>,
        subnets: &IndexMap<PrincipalId, Subnet>,
        ensure_assigned: bool,
    ) -> Vec<(PrincipalId, String, Vec<Node>)> {
        let nodes_in_subnets = subnets
            .values()
            .flat_map(|s| s.nodes.iter().map(|n| (n.principal, n)))
            .collect::<AHashMap<_, _>>();

        node_operators_all
            .iter()
            .filter_map(|(operator_id, operator)| {
                all_nodes_grouped_by_operator.get(operator_id).and_then(|operator_nodes| {
                    let condition = if ensure_assigned {
                        operator_nodes.iter().all(|node| !nodes_in_subnets.contains_key(&node.id))
                    } else {
                        operator_nodes.iter().all(|node| nodes_in_subnets.contains_key(&node.id))
                    };

                    if condition {
                        let nodes = if ensure_assigned {
                            available_nodes_grouped_by_operator
                                .get(operator_id)
                                .unwrap_or(&vec![])
                                .iter()
                                .map(|node| nodes_all.get(&node.id).expect("Node should exist").clone())
                                .collect::<Vec<_>>()
                        } else {
                            operator_nodes
                                .iter()
                                .filter_map(|node| nodes_in_subnets.get(&node.id))
                                .map(|n| (*n).clone())
                                .collect::<Vec<_>>()
                        };

                        if nodes.is_empty() || (!ensure_assigned && nodes.len() < 2) {
                            None
                        } else {
                            Some((
                                *operator_id,
                                operator.datacenter.as_ref().map(|dc| dc.name.clone()).unwrap_or_default(),
                                nodes,
                            ))
                        }
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    async fn get_best_change_for_operator(
        &self,
        subnets: &IndexMap<PrincipalId, Subnet>,
        available_nodes: &[decentralization::network::Node],
        health_of_nodes: &IndexMap<PrincipalId, HealthStatus>,
        node: &Node,
        ensure_assigned: bool,
        cordoned_features: Vec<NodeFeaturePair>,
    ) -> Option<SubnetChangeResponse> {
        let decentr_node = decentralization::network::Node::from(node);
        let mut best_change: Option<SubnetChangeResponse> = None;

        for subnet in subnets.values() {
            let subnet = DecentralizedSubnet::from(subnet);
            let subnet_id_short = subnet.id.to_string().split_once('-').unwrap().0.to_string();
            let change_request = if ensure_assigned {
                SubnetChangeRequest::new(subnet, available_nodes.to_vec(), vec![decentr_node.clone()], vec![], vec![]).resize(
                    0,
                    1,
                    0,
                    health_of_nodes,
                    cordoned_features.clone(),
                )
            } else {
                SubnetChangeRequest::new(subnet, available_nodes.to_vec(), vec![], vec![decentr_node.clone()], vec![]).resize(
                    1,
                    0,
                    0,
                    health_of_nodes,
                    cordoned_features.clone(),
                )
            };

            if let Ok(change) = change_request {
                let change_response = SubnetChangeResponse::new(
                    &change,
                    health_of_nodes,
                    Some(if ensure_assigned {
                        format!("The node operator {} currently does not have nodes in any subnet. To gain insights into the stability of the nodes of this node operator, we propose to add one of the operator's nodes to subnet {}.",
                                node.operator.principal.to_string().split_once('-').unwrap().0,
                                subnet_id_short)
                    } else {
                        format!("The node operator {} currently has all nodes assigned to subnets. We propose to remove one of the operator's nodes from subnet {} to optimize overall network topology, since subnet decentralization does not worsen upon node removal. This way the same node can be assigned to subnet where it would improve decentralization.",
                                node.operator.principal.to_string().split_once('-').unwrap().0,
                                subnet_id_short)
                    }),
                );

                if change_response.penalties_after_change <= change_response.penalties_before_change
                    && change_response.score_after >= change_response.score_before
                {
                    match best_change.clone() {
                        Some(best) => {
                            if change_response.penalties_after_change < best.penalties_after_change
                                || (change_response.penalties_after_change == best.penalties_after_change
                                    && change_response.score_after > best.score_after)
                            {
                                best_change = Some(change_response);
                            }
                        }
                        None => {
                            best_change = Some(change_response);
                        }
                    }
                }
            }
        }

        best_change
    }

    pub async fn network_ensure_operator_nodes(
        &self,
        forum_post_link: Option<String>,
        skip_subnets: &[String],
        ensure_assigned: bool,
    ) -> anyhow::Result<Vec<RunnerProposal>> {
        let mut subnets = self.get_subnets(skip_subnets).await?;
        let (mut available_nodes, health_of_nodes) = self.get_available_and_healthy_nodes().await?;
        let all_node_operators = self.registry.operators().await?;
        let all_nodes = self.registry.nodes().await?;
        let all_nodes_grouped_by_operator = all_nodes
            .values()
            .map(decentralization::network::Node::from)
            .into_group_map_by(|node| all_nodes.get(&node.id).expect("Node should exist").operator.principal);
        let available_nodes_grouped_by_operator = available_nodes
            .iter()
            .map(|n| (*n).clone())
            .into_group_map_by(|node| all_nodes.get(&node.id).expect("Node should exist").operator.principal);
        let cordoned_features = self.cordoned_features_fetcher.fetch().await.unwrap_or_else(|e| {
            warn!("Failed to fetch cordoned features with error: {:?}", e);
            warn!("Will continue running as if no features were cordoned");
            vec![]
        });

        let operators_to_optimize = self.get_operators_to_optimize(
            &all_node_operators,
            &all_nodes_grouped_by_operator,
            &available_nodes_grouped_by_operator,
            &all_nodes,
            &subnets,
            ensure_assigned,
        );

        info!(
            "Checked {} and found {} node operators with all nodes {}, after skipping {} subnets",
            all_node_operators.len(),
            operators_to_optimize.len(),
            if ensure_assigned { "unassigned" } else { "assigned" },
            skip_subnets.len()
        );

        if !operators_to_optimize.is_empty() {
            info!(
                "Will try to optimize the assignments of nodes for {} operators: {:#?}",
                operators_to_optimize.len(),
                operators_to_optimize
                    .iter()
                    .map(|(id, _, _)| id.to_string().split_once('-').unwrap().0.to_string())
                    .collect::<Vec<_>>()
            );
        }

        let mut changes = vec![];
        for (operator_id, dc, healthy_operator_nodes) in operators_to_optimize {
            let node = healthy_operator_nodes.first().expect("At least one node should be present");
            let best_change = self
                .get_best_change_for_operator(
                    &subnets,
                    &available_nodes,
                    &health_of_nodes,
                    node,
                    ensure_assigned,
                    cordoned_features.clone(),
                )
                .await;

            if let Some(change) = best_change {
                info!(
                    "{} node {} of the operator {} in DC {} {} subnet {}",
                    if ensure_assigned { "Adding" } else { "Removing" },
                    node.principal.to_string().split_once('-').unwrap().0,
                    operator_id.to_string().split_once('-').unwrap().0,
                    dc,
                    if ensure_assigned { "to" } else { "from" },
                    change
                        .subnet_id
                        .expect("Subnet ID should be present")
                        .to_string()
                        .split_once('-')
                        .unwrap()
                        .0,
                );
                changes.push(
                    self.run_membership_change(change.clone(), replace_proposal_options(&change, forum_post_link.clone()).await?)
                        .await?,
                );
                subnets.shift_remove(&change.subnet_id.expect("Subnet ID should be present"));
                available_nodes.retain(|n| n.id != node.principal);
            } else {
                warn!(
                    "{} node {} of the operator {} in DC {} would worsen decentralization in all subnets!",
                    if ensure_assigned { "Adding" } else { "Removing" },
                    node.principal.to_string().split_once('-').unwrap().0,
                    operator_id.to_string().split_once('-').unwrap().0,
                    dc,
                );
            }
        }
        Ok(changes)
    }

    pub async fn network_ensure_operator_nodes_assigned(
        &self,
        forum_post_link: Option<String>,
        skip_subnets: &[String],
    ) -> anyhow::Result<Vec<RunnerProposal>> {
        self.network_ensure_operator_nodes(forum_post_link, skip_subnets, true).await
    }

    pub async fn network_ensure_operator_nodes_unassigned(
        &self,
        forum_post_link: Option<String>,
        skip_subnets: &[String],
    ) -> anyhow::Result<Vec<RunnerProposal>> {
        self.network_ensure_operator_nodes(forum_post_link, skip_subnets, false).await
    }

    pub async fn decentralization_change(
        &self,
        change: &ChangeSubnetMembershipPayload,
        override_subnet_nodes: Option<Vec<PrincipalId>>,
        summary: Option<String>,
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
        let subnet_mid = subnet_before.without_nodes(&removed_nodes).map_err(|e| anyhow::anyhow!(e))?;

        // Now simulate node addition
        let added_nodes = self.registry.get_decentralized_nodes(&change.get_added_node_ids()).await?;

        let subnet_after = subnet_mid.with_nodes(&added_nodes);

        let subnet_change = SubnetChange {
            subnet_id: subnet_after.id,
            old_nodes: nodes_before,
            new_nodes: subnet_after.nodes,
            added_nodes: added_nodes.clone(),
            removed_nodes: removed_nodes.clone(),
            ..Default::default()
        };
        println!("{}", SubnetChangeResponse::new(&subnet_change, &health_of_nodes, summary));
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

        let change = &change_request.rescue(&health_of_nodes, self.cordoned_features_fetcher.fetch().await?)?;
        let change = SubnetChangeResponse::new(change, &health_of_nodes, Some("Recovering subnet".to_string()));

        if change.node_ids_added.is_empty() && change.node_ids_removed.is_empty() {
            return Ok(None);
        }

        self.run_membership_change(change.clone(), replace_proposal_options(&change, forum_post_link).await?)
            .await
            .map(Some)
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
                node_ids_add: change.node_ids_added,
                node_ids_remove: change.node_ids_removed,
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

pub async fn replace_proposal_options(change: &SubnetChangeResponse, forum_post_link: Option<String>) -> anyhow::Result<ic_admin::ProposeOptions> {
    let subnet_id = change.subnet_id.ok_or_else(|| anyhow::anyhow!("subnet_id is required"))?.to_string();

    let replace_target = if change.node_ids_added.len() > 1 || change.node_ids_removed.len() > 1 {
        "nodes"
    } else {
        "a node"
    };
    let subnet_id_short = subnet_id.split('-').next().unwrap();

    let change_desc = if change.node_ids_added.len() == change.node_ids_removed.len() {
        format!("Replace {} in subnet {}", replace_target, subnet_id_short)
    } else {
        format!("Resize subnet {}", subnet_id_short)
    };

    let forum_post_link = match forum_post_link {
        Some(_) => forum_post_link,
        None => {
            let base_forum_post_link = format!("https://forum.dfinity.org/t/subnet-management-{}", subnet_id_short);
            let links_to_check = vec![
                format!("{}-nns", base_forum_post_link),
                format!("{}-ii", base_forum_post_link),
                format!("{}-application", base_forum_post_link),
                format!("{}-application-sns", base_forum_post_link),
                format!("{}-fiduciary", base_forum_post_link),
                format!("{}-system-bitcoin", base_forum_post_link),
                format!("{}-european", base_forum_post_link),
            ];
            let mut found_forum_post_link = None;
            for link in links_to_check {
                if get(&link).await?.status().is_success() {
                    found_forum_post_link = Some(link);
                    break;
                }
            }
            found_forum_post_link
        }
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
    sudo apt-get install -y curl && curl --proto '=https' --tlsv1.2 -sSLO https://raw.githubusercontent.com/dfinity/ic/{version}/ci/tools/repro-check.sh && chmod +x repro-check.sh && ./repro-check.sh -c {version} --guestos
    ```

    The two SHA256 sums printed above from a) the downloaded CDN image and b) the locally built image,
    must be identical, and must match the SHA256 from the payload of the NNS proposal.

    While not required for this NNS proposal, as we are only electing a new GuestOS version here, you have the option to verify the build reproducibility of the HostOS by passing `--hostos` to the script above instead of `--guestos`, or the SetupOS by passing `--setupos`.

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
