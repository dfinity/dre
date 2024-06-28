use crate::ic_admin;
use crate::ic_admin::ProposeOptions;
use crate::operations::hostos_rollout::{HostosRollout, HostosRolloutResponse, NodeGroupUpdate};
use crate::ops_subnet_node_replace;
use decentralization::network::{AvailableNodesQuerier, SubnetChange, SubnetQuerier, SubnetQueryBy};
use decentralization::network::{NetworkHealRequest, TopologyManager};
use decentralization::subnets::{MembershipReplace, NodesRemover, ReplaceTarget};
use decentralization::SubnetChangeResponse;
use futures::future::join_all;
use futures::TryFutureExt;
use futures_util::future::try_join;
use ic_base_types::PrincipalId;
use ic_management_backend::proposal::ProposalAgent;
use ic_management_backend::public_dashboard::query_ic_dashboard_list;
use ic_management_backend::registry::{self, RegistryState};
use ic_management_backend::{health, health::HealthStatusQuerier};
use ic_management_types::{Artifact, Network, Node, NodeFeature, NodeProvidersResponse};
use ic_management_types::{NetworkError, TopologyChangePayload};
use itertools::Itertools;
use log::{info, warn};
use registry_canister::mutations::do_change_subnet_membership::ChangeSubnetMembershipPayload;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;
use tabled::builder::Builder;
use tabled::settings::Style;

pub struct Runner {
    pub ic_admin: ic_admin::IcAdminWrapper,
    registry: RefCell<Option<Arc<RegistryState>>>,
    network: Network,
}

impl Runner {
    pub async fn registry(&self) -> Arc<RegistryState> {
        {
            if let Some(ref registry) = *self.registry.borrow() {
                return Arc::clone(registry);
            }
        }

        // Create a new registry state
        let mut new_registry = Arc::new(registry::RegistryState::new(&self.network, true).await);

        // Fetch node providers
        let node_providers = query_ic_dashboard_list::<NodeProvidersResponse>(&self.network, "v3/node-providers")
            .await
            .expect("Failed to get node providers")
            .node_providers;

        // Update node details
        Arc::get_mut(&mut new_registry)
            .expect("Failed to get mutable reference to new registry")
            .update_node_details(&node_providers)
            .await
            .expect("Failed to update node details");

        // Replace the registry in self with the new registry state
        self.registry.replace(Some(Arc::clone(&new_registry)));

        // Return the Arc to the new registry state
        new_registry
    }

    pub async fn new(ic_admin: ic_admin::IcAdminWrapper, network: &Network) -> anyhow::Result<Self> {
        Ok(Self {
            ic_admin,
            registry: RefCell::new(None),
            network: network.clone(),
        })
    }

    pub async fn deploy(&self, subnet: &PrincipalId, version: &str, dry_run: bool) -> anyhow::Result<()> {
        self.ic_admin
            .propose_run(
                ic_admin::ProposeCommand::DeployGuestosToAllSubnetNodes {
                    subnet: *subnet,
                    version: version.to_string(),
                },
                ic_admin::ProposeOptions {
                    title: format!("Update subnet {subnet} to GuestOS version {version}").into(),
                    summary: format!("Update subnet {subnet} to GuestOS version {version}").into(),
                    motivation: None,
                },
                dry_run,
            )
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(())
    }

    pub fn as_automation(self) -> Self {
        Self {
            ic_admin: self.ic_admin.as_automation(),
            ..self
        }
    }

    pub async fn subnet_resize(
        &self,
        request: ic_management_types::requests::SubnetResizeRequest,
        motivation: String,
        verbose: bool,
        dry_run: bool,
    ) -> anyhow::Result<()> {
        let subnet = request.subnet;
        let change = self
            .registry()
            .await
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
            self.run_membership_change(change.clone(), ops_subnet_node_replace::replace_proposal_options(&change)?, dry_run)
                .await
        } else {
            let action = if change.added.len() < change.removed.len() {
                "Removing nodes from"
            } else {
                "Adding nodes to"
            };
            self.run_membership_change(
                change,
                ProposeOptions {
                    title: format!("{action} subnet {subnet}").into(),
                    summary: format!("{action} subnet {subnet}").into(),
                    motivation: motivation.clone().into(),
                },
                dry_run,
            )
            .await
        }
    }

    pub async fn subnet_create(
        &self,
        request: ic_management_types::requests::SubnetCreateRequest,
        motivation: String,
        verbose: bool,
        dry_run: bool,
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
            .registry()
            .await
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
        let replica_version = replica_version.unwrap_or(self.registry().await.nns_replica_version().await?);

        self.ic_admin
            .propose_run(
                ic_admin::ProposeCommand::CreateSubnet {
                    node_ids: subnet_creation_data.added,
                    replica_version,
                    other_args,
                },
                ic_admin::ProposeOptions {
                    title: Some("Creating new subnet".into()),
                    summary: Some("# Creating new subnet with nodes: ".into()),
                    motivation: Some(motivation.clone()),
                },
                dry_run,
            )
            .await?;
        Ok(())
    }

    /// Simulates replacement of nodes in a subnet.
    /// There are multiple ways to replace nodes. For instance:
    ///    1. Setting `heal` to `true` in the request to replace unhealthy nodes
    ///    2. Replace `optimize` nodes to optimize subnet decentralization.
    ///    3. Explicitly add or remove nodes from the subnet specifying their
    /// Principals.
    ///
    /// All nodes in the request must belong to exactly one subnet.
    pub async fn membership_replace(&self, request: MembershipReplace, verbose: bool, dry_run: bool) -> anyhow::Result<()> {
        let mut motivations: Vec<String> = vec![];
        let health_client = health::HealthClient::new(self.registry().await.network());
        let registry_nodes = self.registry().await.nodes();
        let change_request = match &request.target {
            ReplaceTarget::Subnet(subnet) => self.registry().await.modify_subnet_nodes(SubnetQueryBy::SubnetId(*subnet)).await?,
            ReplaceTarget::Nodes {
                nodes: nodes_to_replace,
                motivation,
            } => {
                motivations.push(motivation.clone());
                let nodes_to_replace = nodes_to_replace
                    .iter()
                    .filter_map(|n| registry_nodes.get(n))
                    .map(decentralization::network::Node::from)
                    .collect::<Vec<_>>();
                self.registry()
                    .await
                    .modify_subnet_nodes(SubnetQueryBy::NodeList(nodes_to_replace))
                    .await?
            }
        }
        .excluding_from_available(request.exclude.clone().unwrap_or_default())
        .including_from_available(request.only.clone())
        .including_from_available(request.include.clone().unwrap_or_default())
        .with_min_nakamoto_coefficients(request.min_nakamoto_coefficients.clone());
        let subnet_health: BTreeMap<PrincipalId, ic_management_types::Status> = health_client.subnet(change_request.subnet().id).await?;

        let change = request.replace(subnet_health, registry_nodes, change_request).await?;

        if verbose {
            if let Some(run_log) = &change.run_log {
                println!("{}\n", run_log.join("\n"));
            }
        }
        println!("{}", change);

        if change.added.is_empty() && change.removed.is_empty() {
            return Ok(());
        }
        self.run_membership_change(change.clone(), ops_subnet_node_replace::replace_proposal_options(&change)?, dry_run)
            .await
    }

    async fn run_membership_change(&self, change: SubnetChangeResponse, options: ProposeOptions, dry_run: bool) -> anyhow::Result<()> {
        let subnet_id = change.subnet_id.ok_or_else(|| anyhow::anyhow!("subnet_id is required"))?;
        let pending_action = self
            .registry()
            .await
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
                ic_admin::ProposeCommand::ChangeSubnetMembership {
                    subnet_id,
                    node_ids_add: change.added.clone(),
                    node_ids_remove: change.removed.clone(),
                },
                options,
                dry_run,
            )
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }

    pub async fn prepare_versions_to_retire(&self, release_artifact: &Artifact, edit_summary: bool) -> anyhow::Result<(String, Option<Vec<String>>)> {
        let retireable_versions = self.registry().await.retireable_versions(release_artifact).await?;
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

    async fn nodes_by_dc(&self, nodes: Vec<Node>) -> BTreeMap<String, Vec<(String, String)>> {
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

    pub async fn hostos_rollout_nodes(
        &self,
        node_group: NodeGroupUpdate,
        version: &String,
        exclude: &Option<Vec<PrincipalId>>,
    ) -> anyhow::Result<Option<(Vec<PrincipalId>, String)>> {
        let elected_versions = self.registry().await.blessed_versions(&Artifact::HostOs).await.unwrap();
        if !elected_versions.contains(&version.to_string()) {
            return Err(anyhow::anyhow!(format!(
                "The version {} has not being elected.\nVersions elected are: {:?}",
                version, elected_versions,
            )));
        }
        let hostos_rollout = HostosRollout::new(
            self.registry().await.nodes(),
            self.registry().await.subnets(),
            &self.registry().await.network(),
            ProposalAgent::new(self.registry().await.get_nns_urls()),
            version,
            exclude,
        );

        match hostos_rollout.execute(node_group).await? {
            HostosRolloutResponse::Ok(nodes_to_update, maybe_subnets_affected) => {
                let mut summary = "## List of nodes\n".to_string();
                let mut builder_dc = Builder::default();
                let nodes_by_dc = self.nodes_by_dc(nodes_to_update.clone()).await;
                builder_dc.push_record(["dc", "node_id", "subnet"]);
                nodes_by_dc.into_iter().for_each(|(dc, nodes_with_sub)| {
                    builder_dc.push_record([
                        dc,
                        nodes_with_sub.iter().map(|(p, _)| p.to_string()).join("\n"),
                        nodes_with_sub
                            .iter()
                            .map(|(_, s)| s.to_string().split('-').next().unwrap().to_string())
                            .join("\n"),
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
    pub async fn hostos_rollout(
        &self,
        nodes: Vec<PrincipalId>,
        version: &str,
        dry_run: bool,
        maybe_summary: Option<String>,
        as_automation: bool,
    ) -> anyhow::Result<()> {
        let ic_admin = if as_automation {
            self.ic_admin.clone().as_automation()
        } else {
            self.ic_admin.clone()
        };

        let title = format!("Set HostOS version: {version} on {} nodes", nodes.clone().len());
        ic_admin
            .propose_run(
                ic_admin::ProposeCommand::DeployHostosToSomeNodes {
                    nodes: nodes.clone(),
                    version: version.to_string(),
                },
                ic_admin::ProposeOptions {
                    title: title.clone().into(),
                    summary: maybe_summary.unwrap_or(title).into(),
                    motivation: None,
                },
                dry_run,
            )
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        println!("Submitted proposal to updated the following nodes:\n{:?}", nodes);

        Ok(())
    }

    pub async fn remove_nodes(&self, nodes_remover: NodesRemover, dry_run: bool) -> anyhow::Result<()> {
        let health_client = health::HealthClient::new(self.registry().await.network());
        let (healths, nodes_with_proposals) = try_join(health_client.nodes(), self.registry().await.nodes_with_proposals()).await?;
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
                dry_run,
            )
            .await?;
        Ok(())
    }

    pub async fn network_heal(&self, max_replaceable_nodes_per_sub: Option<usize>, _verbose: bool, simulate: bool) -> Result<(), anyhow::Error> {
        let health_client = health::HealthClient::new(self.registry().await.network());
        let subnets = self.registry().await.subnets();
        let (available_nodes, healths) = try_join(
            self.registry().await.available_nodes().map_err(anyhow::Error::from),
            health_client.nodes(),
        )
        .await?;

        let subnets_change_response: Vec<SubnetChangeResponse> = NetworkHealRequest::new(subnets, max_replaceable_nodes_per_sub)
            .heal_and_optimize(available_nodes, healths)
            .await?;
        subnets_change_response.iter().for_each(|change| println!("{}", change));

        let errors = join_all(subnets_change_response.iter().map(|subnet_change_response| async move {
            self.run_membership_change(
                subnet_change_response.clone(),
                ops_subnet_node_replace::replace_proposal_options(subnet_change_response)?,
                simulate,
            )
            .await
            .map_err(|e| {
                println!("{}", e);
                e
            })
        }))
        .await
        .into_iter()
        .filter_map(|f| f.err())
        .collect::<Vec<_>>();
        if !errors.is_empty() {
            anyhow::bail!("Errors: {:?}", errors);
        }

        Ok(())
    }

    pub async fn decentralization_change(&self, change: &ChangeSubnetMembershipPayload) -> Result<(), anyhow::Error> {
        if let Some(id) = change.get_subnet() {
            let subnet_before = self
                .registry()
                .await
                .subnet(SubnetQueryBy::SubnetId(id))
                .await
                .map_err(|e| anyhow::anyhow!(e))?;
            let nodes_before = subnet_before.nodes.clone();

            let added_nodes = self.registry().await.get_decentralized_nodes(&change.get_added_node_ids());
            let removed_nodes = self.registry().await.get_decentralized_nodes(&change.get_added_node_ids());

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

    pub async fn subnet_rescue(&self, subnet: &PrincipalId, keep_nodes: Option<Vec<String>>, dry_run: bool) -> anyhow::Result<()> {
        let change_request = self
            .registry()
            .await
            .modify_subnet_nodes(SubnetQueryBy::SubnetId(*subnet))
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        let change_request = if let Some(keep_nodes) = keep_nodes {
            change_request.keeping_from_used(keep_nodes)
        } else {
            change_request
        };

        let change = SubnetChangeResponse::from(&change_request.rescue().map_err(|e| anyhow::anyhow!(e))?);
        println!("{}", change);

        if change.added.is_empty() && change.removed.is_empty() {
            return Ok(());
        }

        self.run_membership_change(change.clone(), ops_subnet_node_replace::replace_proposal_options(&change)?, dry_run)
            .await
    }
}
