use crate::clients::DashboardBackendClient;
use crate::ic_admin::ProposeOptions;
use crate::operations::hostos_rollout::{HostosRollout, HostosRolloutResponse, NodeGroupUpdate};
use crate::ops_subnet_node_replace;
use crate::{ic_admin, local_unused_port};
use decentralization::SubnetChangeResponse;
use ic_base_types::PrincipalId;
use ic_management_backend::proposal::ProposalAgent;
use ic_management_backend::public_dashboard::query_ic_dashboard_list;
use ic_management_backend::registry::{self, RegistryState};
use ic_management_types::requests::NodesRemoveRequest;
use ic_management_types::{Artifact, Network, Node, NodeFeature, NodeProvidersResponse};
use itertools::Itertools;
use log::{info, warn};
use std::collections::BTreeMap;
use tabled::builder::Builder;
use tabled::settings::Style;

pub struct Runner {
    pub ic_admin: ic_admin::IcAdminWrapper,
    dashboard_backend_client: DashboardBackendClient,
    registry: RegistryState,
}

impl Runner {
    pub async fn deploy(&self, subnet: &PrincipalId, version: &str, simulate: bool) -> anyhow::Result<()> {
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
                simulate,
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
        simulate: bool,
    ) -> anyhow::Result<()> {
        let subnet = request.subnet;
        let change = self.dashboard_backend_client.subnet_resize(request).await?;
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
            self.run_membership_change(change.clone(), ops_subnet_node_replace::replace_proposal_options(&change)?, simulate)
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
                simulate,
            )
            .await
        }
    }

    pub async fn subnet_create(
        &self,
        request: ic_management_types::requests::SubnetCreateRequest,
        motivation: String,
        verbose: bool,
        simulate: bool,
        replica_version: Option<String>,
        other_args: Vec<String>,
        help_other_args: bool,
    ) -> anyhow::Result<()> {
        if help_other_args {
            println!("The following additional arguments are available for the `subnet create` command:");
            println!("{}", self.ic_admin.grep_subcommand_arguments("propose-to-create-subnet"));
            return Ok(());
        }
        let subnet_creation_data = self.dashboard_backend_client.subnet_create(request).await?;
        if verbose {
            if let Some(run_log) = &subnet_creation_data.run_log {
                println!("{}\n", run_log.join("\n"));
            }
        }
        println!("{}", subnet_creation_data);

        let replica_version = replica_version.unwrap_or(
            self.dashboard_backend_client
                .get_nns_replica_version()
                .await
                .expect("Failed to get a GuestOS version of the NNS subnet"),
        );

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
                simulate,
            )
            .await?;
        Ok(())
    }

    pub async fn membership_replace(
        &self,
        request: ic_management_types::requests::MembershipReplaceRequest,
        verbose: bool,
        simulate: bool,
    ) -> anyhow::Result<()> {
        let change = self.dashboard_backend_client.membership_replace(request).await?;
        if verbose {
            if let Some(run_log) = &change.run_log {
                println!("{}\n", run_log.join("\n"));
            }
        }
        println!("{}", change);

        if change.added.is_empty() && change.removed.is_empty() {
            return Ok(());
        }
        self.run_membership_change(change.clone(), ops_subnet_node_replace::replace_proposal_options(&change)?, simulate)
            .await
    }

    async fn run_membership_change(&self, change: SubnetChangeResponse, options: ProposeOptions, simulate: bool) -> anyhow::Result<()> {
        let subnet_id = change.subnet_id.ok_or_else(|| anyhow::anyhow!("subnet_id is required"))?;
        let pending_action = self.dashboard_backend_client.subnet_pending_action(subnet_id).await?;
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
                simulate,
            )
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }

    pub async fn new_with_network_and_backend_port(ic_admin: ic_admin::IcAdminWrapper, network: &Network, backend_port: u16) -> anyhow::Result<Self> {
        let backend_url = format!("http://localhost:{}/", backend_port);

        let dashboard_backend_client = DashboardBackendClient::new_with_backend_url(backend_url);
        let mut registry = registry::RegistryState::new(network, true).await;
        let node_providers = query_ic_dashboard_list::<NodeProvidersResponse>("v3/node-providers")
            .await?
            .node_providers;
        registry.update_node_details(&node_providers).await?;

        Ok(Self {
            ic_admin,
            dashboard_backend_client,
            // TODO: Remove once DREL-118 completed.
            registry,
        })
    }

    pub async fn new(ic_admin: ic_admin::IcAdminWrapper, network: &Network) -> anyhow::Result<Self> {
        // TODO: Remove once DREL-118 completed.
        let backend_port = local_unused_port();
        let backend_url = format!("http://localhost:{}/", backend_port);
        let dashboard_backend_client = DashboardBackendClient::new_with_backend_url(backend_url);

        let mut registry = registry::RegistryState::new(network, true).await;
        let node_providers = query_ic_dashboard_list::<NodeProvidersResponse>("v3/node-providers")
            .await?
            .node_providers;
        registry.update_node_details(&node_providers).await?;
        Ok(Self {
            ic_admin,
            dashboard_backend_client,
            registry,
        })
    }

    pub async fn prepare_versions_to_retire(&self, release_artifact: &Artifact, edit_summary: bool) -> anyhow::Result<(String, Option<Vec<String>>)> {
        let retireable_versions = self.dashboard_backend_client.get_retireable_versions(release_artifact).await?;

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
        let elected_versions = self.registry.blessed_versions(&Artifact::HostOs).await.unwrap();
        if !elected_versions.contains(&version.to_string()) {
            return Err(anyhow::anyhow!(format!(
                "The version {} has not being elected.\nVersions elected are: {:?}",
                version, elected_versions,
            )));
        }
        let hostos_rollout = HostosRollout::new(
            self.registry.nodes(),
            self.registry.subnets(),
            &self.registry.network(),
            ProposalAgent::new(self.registry.get_nns_urls()),
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
    pub async fn hostos_rollout(&self, nodes: Vec<PrincipalId>, version: &str, simulate: bool, maybe_summary: Option<String>) -> anyhow::Result<()> {
        let title = format!("Set HostOS version: {version} on {} nodes", nodes.clone().len());
        self.ic_admin
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
                simulate,
            )
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        println!("Submitted proposal to updated the following nodes:\n{:?}", nodes);

        Ok(())
    }

    pub async fn remove_nodes(&self, request: NodesRemoveRequest, simulate: bool) -> anyhow::Result<()> {
        let node_remove_response = self.dashboard_backend_client.remove_nodes(request).await?;
        let mut node_removals = node_remove_response.removals;
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
                    motivation: node_remove_response.motivation.into(),
                },
                simulate,
            )
            .await?;
        Ok(())
    }
}
