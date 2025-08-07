use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use ahash::AHashMap;
use decentralization::network::CordonedFeature;
use decentralization::network::DecentralizedSubnet;
use decentralization::network::NetworkHealRequest;
use decentralization::network::SubnetChange;
use decentralization::network::SubnetChangeRequest;
use decentralization::network::SubnetQueryBy;
use decentralization::subnets::NodesRemover;
use decentralization::SubnetChangeResponse;
use futures::future::try_join3;
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
use tabled::builder::Builder;
use tabled::settings::Style;

use crate::artifact_downloader::ArtifactDownloader;
use crate::cordoned_feature_fetcher::CordonedFeatureFetcher;
use crate::ic_admin::IcAdminProposal;
use crate::ic_admin::{self};
use crate::ic_admin::{IcAdminProposalCommand, IcAdminProposalOptions};
use crate::operations::hostos_rollout::HostosRollout;
use crate::operations::hostos_rollout::HostosRolloutResponse;
use crate::operations::hostos_rollout::NodeGroupUpdate;
use crate::target_topology::TargetTopology;

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

    pub async fn deploy(&self, subnet: &PrincipalId, version: &str) -> anyhow::Result<IcAdminProposal> {
        Ok(IcAdminProposal::new(
            IcAdminProposalCommand::DeployGuestosToAllSubnetNodes {
                subnet: *subnet,
                version: version.to_owned(),
            },
            IcAdminProposalOptions {
                title: format!("Update subnet {subnet} to GuestOS version {version}").into(),
                summary: format!("Update subnet {subnet} to GuestOS version {version}").into(),
                motivation: None,
            },
        ))
    }

    pub async fn health_of_nodes(&self) -> anyhow::Result<IndexMap<PrincipalId, HealthStatus>> {
        self.health_client.nodes().await
    }

    pub async fn subnet_create(
        &self,
        request: ic_management_types::requests::SubnetCreateRequest,
        motivation: String,
        replica_version: Option<String>,
        other_args: Vec<String>,
    ) -> anyhow::Result<Option<IcAdminProposal>> {
        let all_nodes = self.registry.nodes().await?.values().cloned().collect_vec();
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
                &all_nodes,
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

        let summary_nodes = subnet_creation_data
            .node_ids_added
            .iter()
            .map(|i| i.to_string().split_once("-").unwrap().0.to_string())
            .join(", ");

        Ok(Some(IcAdminProposal::new(
            IcAdminProposalCommand::CreateSubnet {
                node_ids: subnet_creation_data.node_ids_added,
                replica_version,
                other_args,
            },
            IcAdminProposalOptions {
                title: Some("Creating a new subnet".into()),
                summary: Some(format!("# Creating a new subnet with nodes: [{}]", summary_nodes)),
                motivation: Some(motivation),
            },
        )))
    }

    fn should_submit(&self, change: &SubnetChangeResponse) -> bool {
        if self.verbose {
            if let Some(run_log) = &change.run_log {
                println!("{}\n", run_log.join("\n"));
            }
        }

        if change.node_ids_added.is_empty() && change.node_ids_removed.is_empty() {
            return false;
        }
        true
    }

    pub async fn propose_subnet_change(&self, change: &SubnetChangeResponse) -> anyhow::Result<Option<IcAdminProposal>> {
        if !self.should_submit(&change) {
            return Ok(None);
        }

        self.run_membership_change(change, replace_proposal_options(change)?).await.map(Some)
    }

    pub async fn propose_force_subnet_change(
        &self,
        change: &SubnetChangeResponse,
        target_topology: TargetTopology,
    ) -> anyhow::Result<Option<IcAdminProposal>> {
        if !self.should_submit(&change) {
            return Ok(None);
        }

        Ok(Some(IcAdminProposal::new(
            IcAdminProposalCommand::ChangeSubnetMembership {
                subnet_id: change.subnet_id.ok_or(anyhow::anyhow!("Subnet id is required"))?,
                node_ids_add: change.node_ids_added.to_vec(),
                node_ids_remove: change.node_ids_removed.to_vec(),
            },
            force_replace_proposal_options(change, target_topology)?,
        )))
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
        release_tag: &Option<String>,
        ignore_missing_urls: bool,
        security_fix: bool,
    ) -> anyhow::Result<IcAdminProposal> {
        let update_version = self
            .prepare_to_propose_to_revise_elected_versions(
                release_artifact,
                version,
                release_tag,
                ignore_missing_urls,
                self.prepare_versions_to_retire(release_artifact, false).await.map(|r| r.1)?,
                security_fix,
            )
            .await?;

        Ok(IcAdminProposal::new(
            IcAdminProposalCommand::ReviseElectedVersions {
                release_artifact: update_version.release_artifact.clone(),
                args: update_version.get_update_cmd_args(&update_version.release_artifact),
            },
            IcAdminProposalOptions {
                title: Some(update_version.title),
                summary: Some(update_version.summary.clone()),
                motivation: None,
            },
        ))
    }

    async fn prepare_to_propose_to_revise_elected_versions(
        &self,
        release_artifact: &Artifact,
        version: &str,
        release_tag: &Option<String>,
        ignore_missing_urls: bool,
        retire_versions: Option<Vec<String>>,
        security_fix: bool,
    ) -> anyhow::Result<UpdateVersion> {
        let (update_urls, expected_hash) = self
            .artifact_downloader
            .download_images_and_validate_sha256(release_artifact, version, ignore_missing_urls)
            .await?;

        let summary = match security_fix {
            true => format_security_hotfix(),
            false => format_regular_version_upgrade_summary(version, release_artifact, release_tag)?,
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

    pub fn hostos_rollout(&self, nodes: Vec<PrincipalId>, version: &str, maybe_summary: Option<String>) -> anyhow::Result<IcAdminProposal> {
        let title = format!("Set HostOS version: {version} on {} nodes", nodes.clone().len());

        let nodes_short = nodes
            .iter()
            .map(|p| p.to_string().split('-').next().unwrap().to_string())
            .collect::<Vec<_>>();
        println!("Will submit proposal to update the following nodes: {:?}", nodes_short);
        println!("You will be able to follow the upgrade progress at https://grafana.mainnet.dfinity.network/explore?orgId=1&left=%7B%22datasource%22:%22PE62C54679EC3C073%22,%22queries%22:%5B%7B%22refId%22:%22A%22,%22datasource%22:%7B%22type%22:%22prometheus%22,%22uid%22:%22PE62C54679EC3C073%22%7D,%22editorMode%22:%22code%22,%22expr%22:%22hostos_version%7Bic_node%3D~%5C%22{}%5C%22%7D%5Cn%22,%22legendFormat%22:%22__auto%22,%22range%22:true,%22instant%22:true%7D%5D,%22range%22:%7B%22from%22:%22now-1h%22,%22to%22:%22now%22%7D%7D", nodes_short.iter().map(|n| n.to_string() + ".%2B").join("%7C"));

        Ok(IcAdminProposal::new(
            IcAdminProposalCommand::DeployHostosToSomeNodes {
                nodes: nodes.clone(),
                version: version.to_string(),
            },
            IcAdminProposalOptions {
                title: title.clone().into(),
                summary: maybe_summary.unwrap_or(title).into(),
                motivation: None,
            },
        ))
    }

    pub async fn remove_nodes(&self, nodes_remover: NodesRemover) -> anyhow::Result<IcAdminProposal> {
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
            row.add_cell(nr.node.principal);
            for nf in NodeFeature::variants() {
                row.add_cell(nr.node.get_feature(&nf).expect("Feature should exist"));
            }
            row.add_cell(nr.node.hostname.clone().unwrap_or_else(|| "N/A".to_string()));
            row.add_cell(nr.reason.message());
            table.add_row(row);
        }
        println!("{}", table);

        Ok(IcAdminProposal::new(
            ic_admin::IcAdminProposalCommand::RemoveNodes {
                nodes: node_removals.iter().map(|n| n.node.principal).collect(),
            },
            IcAdminProposalOptions {
                title: "Remove nodes from the network".to_string().into(),
                summary: "Remove nodes from the network".to_string().into(),
                motivation: motivation.into(),
            },
        ))
    }

    pub async fn network_fix(
        &self,
        omit_subnets: &[String],
        omit_nodes: &[String],
        optimize_decentralization: bool,
        remove_cordoned_nodes: bool,
        heal: bool,
    ) -> anyhow::Result<Vec<IcAdminProposal>> {
        let mut errors = vec![];

        // Get the list of subnets, and the list of open proposal for each subnet, if any
        let subnets = self.registry.subnets_and_proposals().await?;
        let subnets_not_skipped = subnets
            .iter()
            .filter(|(subnet_id, _)| !omit_subnets.iter().any(|s| subnet_id.to_string().contains(s)))
            .map(|(subnet_id, subnet)| (*subnet_id, subnet.clone()))
            .collect::<IndexMap<_, _>>();
        let subnets_without_proposals = filter_subnets_without_proposals(subnets_not_skipped);
        let (all_nodes, available_nodes, health_of_nodes) = try_join3(
            self.registry.nodes(),
            self.registry.available_nodes().map_err(anyhow::Error::from),
            self.health_client.nodes(),
        )
        .await?;
        let all_nodes = all_nodes.values().cloned().collect::<Vec<Node>>();
        let available_nodes = available_nodes
            .into_iter()
            .filter(|n| !omit_nodes.iter().any(|s| n.principal.to_string().contains(s)))
            .collect::<Vec<Node>>();

        let cordoned_features = self.cordoned_features_fetcher.fetch().await.unwrap_or_else(|e| {
            warn!("Failed to fetch cordoned features with error: {:?}", e);
            warn!("Will continue running as if no features were cordoned");
            vec![]
        });

        let subnets_change_responses = NetworkHealRequest::new(subnets_without_proposals)
            .fix_and_optimize(
                available_nodes,
                &health_of_nodes,
                cordoned_features,
                &all_nodes,
                optimize_decentralization,
                remove_cordoned_nodes,
                heal,
            )
            .await?;

        let mut changes = vec![];
        for change in &subnets_change_responses {
            let current = self.run_membership_change(change, replace_proposal_options(change)?).await.map_err(|e| {
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

    async fn get_subnets(&self, omit_subnets: &[String]) -> anyhow::Result<IndexMap<PrincipalId, Subnet>> {
        let subnets = self.registry.subnets_and_proposals().await?;
        Ok(subnets
            .iter()
            .filter(|(subnet_id, _)| !omit_subnets.iter().any(|s| subnet_id.to_string().contains(s)))
            .map(|(subnet_id, subnet)| (*subnet_id, subnet.clone()))
            .collect::<IndexMap<_, _>>())
    }

    async fn get_available_and_healthy_nodes(
        &self,
        cordoned_features: &[CordonedFeature],
    ) -> anyhow::Result<(Vec<Node>, IndexMap<PrincipalId, HealthStatus>)> {
        let (available_nodes, node_health) =
            try_join(self.registry.available_nodes().map_err(anyhow::Error::from), self.health_client.nodes()).await?;
        let available_nodes = available_nodes
            .into_iter()
            .filter(|n| !cordoned_features.iter().any(|cf| n.get_feature(&cf.feature).as_ref() == Some(&cf.value)))
            .collect();
        Ok((available_nodes, node_health))
    }

    fn get_operators_to_optimize(
        &self,
        node_operators_all: &IndexMap<PrincipalId, Operator>,
        all_nodes_grouped_by_operator: &HashMap<PrincipalId, Vec<Node>>,
        available_nodes_grouped_by_operator: &HashMap<PrincipalId, Vec<Node>>,
        nodes_in_subnets_or_proposals: &AHashMap<PrincipalId, Node>,
        nodes_all: &IndexMap<PrincipalId, Node>,
        ensure_assigned: bool,
    ) -> Vec<(PrincipalId, String, Vec<Node>)> {
        node_operators_all
            .iter()
            .filter_map(|(operator_id, operator)| {
                all_nodes_grouped_by_operator.get(operator_id).and_then(|operator_nodes| {
                    let should_optimize_operator_nodes = operator_nodes.len() > 2  // For 1-2 nodes don't bother
                        && if ensure_assigned {
                            operator_nodes
                                .iter()
                                .all(|node| !nodes_in_subnets_or_proposals.contains_key(&node.principal))
                        } else {
                            operator_nodes
                                .iter()
                                .all(|node| nodes_in_subnets_or_proposals.contains_key(&node.principal))
                        };

                    if should_optimize_operator_nodes {
                        let nodes = if ensure_assigned {
                            available_nodes_grouped_by_operator
                                .get(operator_id)
                                .unwrap_or(&vec![])
                                .iter()
                                .map(|node| {
                                    nodes_all
                                        .get(&node.principal)
                                        .expect("Available node should be listed within all nodes")
                                        .clone()
                                })
                                .collect::<Vec<_>>()
                        } else {
                            operator_nodes
                                .iter()
                                .filter_map(|node| nodes_in_subnets_or_proposals.get(&node.principal))
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
        available_nodes: &[Node],
        health_of_nodes: &IndexMap<PrincipalId, HealthStatus>,
        node: &Node,
        ensure_assigned: bool,
        cordoned_features: Vec<CordonedFeature>,
        all_nodes: &[Node],
    ) -> Option<SubnetChangeResponse> {
        let mut best_change: Option<SubnetChangeResponse> = None;

        for subnet in subnets.values() {
            let subnet = DecentralizedSubnet::from(subnet);
            let subnet_id_short = subnet.id.to_string().split_once('-').unwrap().0.to_string();
            let change_request = if ensure_assigned {
                SubnetChangeRequest::new(subnet, available_nodes.to_vec(), vec![node.clone()], vec![], vec![]).resize(
                    0,
                    1,
                    0,
                    health_of_nodes,
                    cordoned_features.clone(),
                    all_nodes,
                )
            } else {
                SubnetChangeRequest::new(subnet, available_nodes.to_vec(), vec![], vec![node.clone()], vec![]).resize(
                    1,
                    0,
                    0,
                    health_of_nodes,
                    cordoned_features.clone(),
                    all_nodes,
                )
            };

            if let Ok(change) = change_request {
                let change_response = SubnetChangeResponse::new(
                    &change,
                    health_of_nodes,
                    Some(if ensure_assigned {
                        let np = node.operator.provider.principal.to_string();
                        let np_short = np.split_once('-').unwrap().0;
                        format!("The node operator `{}` (under NP [{}](https://dashboard.internetcomputer.org/provider/{})) has {} nodes in total but currently does not have active nodes in any subnet. To gain insights into the stability of the nodes of this node operator, we propose to add one of the operator's nodes to subnet {}.",
                                node.operator.principal.to_string().split_once('-').unwrap().0,
                                np_short,
                                np,
                                Self::num_nodes_per_operator(all_nodes, &node.operator.principal),
                                subnet_id_short)
                    } else {
                        let np = node.operator.provider.principal.to_string();
                        let np_short = np.split_once('-').unwrap().0;
                        format!("The node operator {} (under NP [{}](https://dashboard.internetcomputer.org/provider/{})) has {} nodes in total and currently has all nodes assigned to subnets. We propose to remove one of the operator's nodes from subnet {} to allow optimization of the overall network topology. The removal of the node from the subnet does not worsen subnet decentralization, and may allow the node to be assigned to subnet where it would improve decentralization.",
                                node.operator.principal.to_string().split_once('-').unwrap().0,
                                np_short,
                                np,
                                Self::num_nodes_per_operator(all_nodes, &node.operator.principal),
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

    pub fn num_nodes_per_operator(all_nodes: &[Node], operator_id: &PrincipalId) -> usize {
        all_nodes.iter().filter(|n| &n.operator.principal == operator_id).count()
    }

    pub async fn network_ensure_operator_nodes(
        &self,
        omit_subnets: &[String],
        omit_nodes: &[String],
        ensure_assigned: bool,
    ) -> anyhow::Result<Vec<IcAdminProposal>> {
        let subnets_not_skipped = self.get_subnets(omit_subnets).await?;
        let mut subnets_that_have_no_proposals = filter_subnets_without_proposals(subnets_not_skipped);
        let cordoned_features = self.cordoned_features_fetcher.fetch().await.unwrap_or_else(|e| {
            warn!("Failed to fetch cordoned features with error: {:?}", e);
            warn!("Will continue running as if no features were cordoned");
            vec![]
        });
        let (available_nodes, health_of_nodes) = self.get_available_and_healthy_nodes(&cordoned_features).await?;
        let mut available_nodes = available_nodes
            .into_iter()
            .filter(|n| !omit_nodes.iter().any(|s| n.principal.to_string().contains(s)))
            .collect::<Vec<Node>>();
        let all_node_operators = self.registry.operators().await?;
        let all_nodes_map = self.registry.nodes().await?;
        let all_nodes = all_nodes_map.values().cloned().collect_vec();
        let all_nodes_grouped_by_operator = all_nodes
            .iter()
            .cloned()
            .into_group_map_by(|node| all_nodes_map.get(&node.principal).expect("Node should exist").operator.principal);
        let available_nodes_grouped_by_operator = available_nodes
            .iter()
            .map(|n| (*n).clone())
            .into_group_map_by(|node| all_nodes_map.get(&node.principal).expect("Node should exist").operator.principal);
        let nodes_with_proposals_grouped_by_operator = self
            .registry
            .nodes_and_proposals()
            .await?
            .iter()
            .filter(|(_, n)| n.proposal.is_some())
            .map(|(_, n)| n.clone())
            .into_group_map_by(|node| node.operator.principal);
        let nodes_in_subnets_or_proposals = nodes_in_subnets_or_proposals(&subnets_that_have_no_proposals, &nodes_with_proposals_grouped_by_operator);

        let operators_to_optimize = self.get_operators_to_optimize(
            &all_node_operators,
            &all_nodes_grouped_by_operator,
            &available_nodes_grouped_by_operator,
            &nodes_in_subnets_or_proposals,
            &all_nodes_map,
            ensure_assigned,
        );

        info!(
            "Checked {} and found {} node operators with all nodes {}, after skipping {} subnets",
            all_node_operators.len(),
            operators_to_optimize.len(),
            if ensure_assigned { "unassigned" } else { "assigned" },
            omit_subnets.len()
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
                    &subnets_that_have_no_proposals,
                    &available_nodes,
                    &health_of_nodes,
                    node,
                    ensure_assigned,
                    cordoned_features.clone(),
                    &all_nodes,
                )
                .await;

            if let Some(change) = best_change {
                if ensure_assigned {
                    // If this is the single assigned node of the removed operator, we should not remove it
                    let node_id_removed = change.node_ids_removed.first().expect("At least one node should be present");
                    let node_removed = all_nodes_map.get(node_id_removed).expect("Node should exist");
                    if Self::num_nodes_per_operator(&all_nodes, &node_removed.operator.principal) < 2 {
                        warn!(
                            "Skipping node {} of the operator {} in DC {} as it is the only node of the operator",
                            node.principal.to_string().split_once('-').unwrap().0,
                            operator_id.to_string().split_once('-').unwrap().0,
                            dc
                        );
                        continue;
                    }
                } else {
                    // Ensuring that some nodes are unassigned ==> prevent that we're making the same problem again by assigning all nodes of the newly picked operator
                    let node_id_added = change.node_ids_added.first().expect("At least one node should be present");
                    let node_added = all_nodes_map.get(node_id_added).expect("Node should exist");
                    if Self::num_nodes_per_operator(&available_nodes, &node_added.operator.principal) < 2 {
                        warn!(
                            "Skipping node {} of the operator {} in DC {} as it does not have enough available nodes",
                            node_added.principal.to_string().split_once('-').unwrap().0,
                            node_added.operator.principal.to_string().split_once('-').unwrap().0,
                            dc
                        );
                        continue;
                    }
                };
                changes.push(self.run_membership_change(&change, replace_proposal_options(&change)?).await?);
                subnets_that_have_no_proposals.shift_remove(&change.subnet_id.expect("Subnet ID should be present"));
                available_nodes.retain(|n| !change.node_ids_added.contains(&n.principal));
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
        omit_subnets: &[String],
        omit_nodes: &[String],
    ) -> anyhow::Result<Vec<IcAdminProposal>> {
        self.network_ensure_operator_nodes(omit_subnets, omit_nodes, true).await
    }

    pub async fn network_ensure_operator_nodes_unassigned(
        &self,
        omit_subnets: &[String],
        omit_nodes: &[String],
    ) -> anyhow::Result<Vec<IcAdminProposal>> {
        self.network_ensure_operator_nodes(omit_subnets, omit_nodes, false).await
    }

    pub async fn decentralization_change(
        &self,
        change: &ChangeSubnetMembershipPayload,
        override_subnet_nodes: Option<Vec<PrincipalId>>,
        summary: Option<String>,
    ) -> anyhow::Result<SubnetChangeResponse> {
        let subnet_before = match override_subnet_nodes {
            Some(nodes) => {
                let nodes = self.registry.get_nodes_from_ids(&nodes).await?;
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
        let removed_nodes = self.registry.get_nodes_from_ids(&change.get_removed_node_ids()).await?;
        let subnet_mid = subnet_before.without_nodes(&removed_nodes).map_err(|e| anyhow::anyhow!(e))?;

        // Now simulate node addition
        let added_nodes = self.registry.get_nodes_from_ids(&change.get_added_node_ids()).await?;

        let subnet_after = subnet_mid.with_nodes(&added_nodes);

        let subnet_change = SubnetChange {
            subnet_id: subnet_after.id,
            old_nodes: nodes_before,
            new_nodes: subnet_after.nodes,
            added_nodes: added_nodes.clone(),
            removed_nodes: removed_nodes.clone(),
            ..Default::default()
        };
        Ok(SubnetChangeResponse::new(&subnet_change, &health_of_nodes, summary))
    }

    pub async fn subnet_rescue(&self, subnet: &PrincipalId, keep_nodes: Option<Vec<String>>) -> anyhow::Result<Option<IcAdminProposal>> {
        let change_request = self
            .registry
            .modify_subnet_nodes(SubnetQueryBy::SubnetId(*subnet))
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        let change_request = match keep_nodes {
            Some(n) => change_request.keeping_from_used(n),
            None => change_request,
        };

        let all_nodes = self.registry.nodes().await?.values().cloned().collect_vec();
        let health_of_nodes = self.health_of_nodes().await?;

        let change = &change_request.rescue(&health_of_nodes, self.cordoned_features_fetcher.fetch().await?, &all_nodes)?;
        let change = SubnetChangeResponse::new(change, &health_of_nodes, Some("Recovering subnet".to_string()));

        if change.node_ids_added.is_empty() && change.node_ids_removed.is_empty() {
            return Ok(None);
        }

        self.run_membership_change(&change, replace_proposal_options(&change)?).await.map(Some)
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

    async fn run_membership_change(&self, change: &SubnetChangeResponse, options: IcAdminProposalOptions) -> anyhow::Result<IcAdminProposal> {
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
        Ok(IcAdminProposal::new(
            IcAdminProposalCommand::ChangeSubnetMembership {
                subnet_id,
                node_ids_add: change.node_ids_added.to_vec(),
                node_ids_remove: change.node_ids_removed.to_vec(),
            },
            options,
        ))
    }

    pub async fn update_unassigned_nodes(&self, nns_subnet_id: &PrincipalId) -> anyhow::Result<Option<IcAdminProposal>> {
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

        Ok(Some(IcAdminProposal::new(
            IcAdminProposalCommand::DeployGuestosToAllUnassignedNodes {
                replica_version: nns.replica_version.clone(),
            },
            IcAdminProposalOptions {
                summary: Some("Update the unassigned nodes to the latest rolled-out version".to_string()),
                motivation: None,
                title: Some("Update all unassigned nodes".to_string()),
            },
        )))
    }
}

fn filter_subnets_without_proposals(subnets: IndexMap<PrincipalId, Subnet>) -> IndexMap<PrincipalId, Subnet> {
    subnets
        .iter()
        .filter(|(subnet_id, subnet)| match &subnet.proposal {
            Some(p) => {
                info!("Skipping subnet {} as it has a pending proposal {}", subnet_id, p.id);
                false
            }
            None => true,
        })
        .map(|(id, subnet)| (*id, subnet.clone()))
        .collect::<IndexMap<_, _>>()
}

fn nodes_in_subnets_or_proposals(
    subnets: &IndexMap<PrincipalId, Subnet>,
    nodes_with_proposals_grouped_by_operator: &HashMap<PrincipalId, Vec<Node>>,
) -> AHashMap<PrincipalId, Node> {
    let nodes_in_subnets = subnets
        .values()
        .flat_map(|s| s.nodes.iter().map(|n| (n.principal, n.clone())))
        .collect::<AHashMap<_, _>>();
    let nodes_in_subnets_or_proposals = nodes_in_subnets
        .into_iter()
        .chain(
            nodes_with_proposals_grouped_by_operator
                .iter()
                .flat_map(|(_, nodes)| nodes.iter().map(|n| (n.principal, n.clone()))),
        )
        .collect::<AHashMap<_, _>>();
    nodes_in_subnets_or_proposals
}

fn title_and_summary(change: &SubnetChangeResponse) -> anyhow::Result<ic_admin::IcAdminProposalOptions> {
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

    Ok(ic_admin::IcAdminProposalOptions {
        title: Some(change_desc.clone()),
        summary: Some(format!("# {change_desc}")),
        // Should be formatted later
        motivation: None,
    })
}

pub fn replace_proposal_options(change: &SubnetChangeResponse) -> anyhow::Result<ic_admin::IcAdminProposalOptions> {
    title_and_summary(&change).map(|opts| ic_admin::IcAdminProposalOptions {
        motivation: Some(format!("{}\n\n{}\n", change.motivation.as_ref().unwrap_or(&String::new()), change)),
        ..opts
    })
}

fn force_replace_proposal_options(
    change: &SubnetChangeResponse,
    target_topology: TargetTopology,
) -> anyhow::Result<ic_admin::IcAdminProposalOptions> {
    let topology_summary = target_topology.summary_for_change(change)?;
    title_and_summary(&change).map(|opts| ic_admin::IcAdminProposalOptions {
        motivation: Some(format!(
            "{}\n\n{}\n",
            change.motivation.as_ref().unwrap_or(&String::new()),
            topology_summary
        )),
        ..opts
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
    pub fn get_update_cmd_args(&self, release_artifact: &Artifact) -> Vec<String> {
        [
            [
                vec![
                    match release_artifact {
                        Artifact::GuestOs => "--replica-version-to-elect".to_string(),
                        Artifact::HostOs => "--hostos-version-to-elect".to_string(),
                    },
                    self.version.to_string(),
                    "--release-package-sha256-hex".to_string(),
                    self.stringified_hash.to_string(),
                    "--release-package-urls".to_string(),
                ],
                self.update_urls.clone(),
            ]
            .concat(),
            match self.versions_to_retire.clone() {
                Some(versions) => [
                    vec![match release_artifact {
                        Artifact::GuestOs => "--replica-versions-to-unelect".to_string(),
                        Artifact::HostOs => "--hostos-versions-to-unelect".to_string(),
                    }],
                    versions,
                ]
                .concat(),
                None => vec![],
            },
        ]
        .concat()
    }
}

pub fn format_regular_version_upgrade_summary(version: &str, release_artifact: &Artifact, release_tag: &Option<String>) -> anyhow::Result<String> {
    let release_tag = match release_tag {
        Some(git_tag) => git_tag,
        None => return Err(anyhow::anyhow!("Release tag is required for non-security versions")),
    };
    let (artifact_str, artifact_name) = match release_artifact {
        Artifact::GuestOs => ("--guestos", "GuestOS"),
        Artifact::HostOs => ("--hostos", "HostOS"),
    };
    let otheros_text = match release_artifact {
        Artifact::GuestOs => {
            r#"

While not required for this NNS proposal, as we are only electing a new GuestOS version here, you have the option to verify the build reproducibility of the HostOS by passing `--hostos` to the script above instead of `--guestos`, or the SetupOS by passing `--setupos`."#
        }
        Artifact::HostOs => {
            r#"

While not required for this NNS proposal, as we are only electing a new HostOS version here, you have the option to verify the build reproducibility of the HostOS by passing `--guestos` to the script above instead of `--hostos`, or the SetupOS by passing `--setupos`."#
        }
    };
    let template = format!(
        r#"Elect new {artifact_name} binary revision [{version}](https://github.com/dfinity/ic/tree/{release_tag})

# Release Notes:

[comment]: <> Remove this block of text from the proposal.
[comment]: <> Then, add the {artifact_name} binary release notes as bullet points here.
[comment]: <> Any [commit ID] within square brackets will auto-link to the specific changeset.

# IC-OS Verification

To build and verify the IC-OS {artifact_name} disk image, after installing curl if necessary (`sudo apt install curl`), run:

```
# From https://github.com/dfinity/ic#verifying-releases
curl -fsSL https://raw.githubusercontent.com/dfinity/ic/master/ci/tools/repro-check | python3 - -c {version} {artifact_str}
```

The two SHA256 sums printed above from a) the downloaded CDN image and b) the locally built image,
must be identical, and must match the SHA256 from the payload of the NNS proposal.{otheros_text}
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

pub fn format_security_hotfix() -> String {
    r#"In accordance with the Security Patch Policy and Procedure that was adopted in proposal [48792](https://dashboard.internetcomputer.org/proposal/48792), the source code that was used to build this release will be exposed at the latest 10 days after the fix is rolled out to all subnets.

The community will be able to retroactively verify the binaries that were rolled out.
"#.lines().map(|l| l.trim()).join("\n")
}
