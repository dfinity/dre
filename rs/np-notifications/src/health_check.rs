use actix_web::{rt::time::sleep, web};
use core::time;
use ic_management_types::NodeProvidersResponse;
use std::sync::mpsc::Sender;

use ic_management_backend::{health::HealthClient, public_dashboard::query_ic_dashboard_list, registry::RegistryState};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::{nodes_status::NodesStatus, notification::Notification, ServiceHealth};

pub struct HealthCheckLoopConfig {
    pub notification_sender: Sender<Notification>,
    pub cancellation_token: CancellationToken,
    pub registry_state: RegistryState,
    pub service_health: web::Data<ServiceHealth>,
}

// There is no real information in the Config, so just print its name as debug
// Necessary for tracing
impl std::fmt::Debug for HealthCheckLoopConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "HealthCheckLoopConfig")
    }
}

#[tracing::instrument]
pub async fn start_health_check_loop(config: HealthCheckLoopConfig) {
    info!("Starting health check loop");
    let network = ic_management_types::Network::new("mainnet", &vec![])
        .await
        .expect("failed to create mainnet network");
    let hc = HealthClient::new(network);
    let mut nodes_status = NodesStatus::from(hc.nodes().await.unwrap());

    let mut rs = config.registry_state;
    // NOTE: The way this is now, we would only update the list of node
    // providers once before the health check loop starts.
    // This is not future proof, and needs to be updated, but we would want to
    // update this list much less frequently than when checking the health of
    // the nodes, since the list will evolve much more slowly.
    // For now, it is acceptable to update this list only when the service
    // reboots. In the worst case, if a provider is not up to date in the list,
    // the program will crash, then restart and update the list, which should
    // fix the issue.
    let node_providers = query_ic_dashboard_list::<NodeProvidersResponse>("v3/node-providers")
        .await
        .unwrap()
        .node_providers;
    loop {
        if config.cancellation_token.is_cancelled() {
            break;
        }
        match hc.nodes().await {
            Ok(new_statuses) => {
                config.service_health.set_health_check_loop_readiness(true);
                // Probably need to change the way we create the notifications there to
                // include the fetching from the registry
                let (new_nodes_status, notifications) = nodes_status.updated_from_map(new_statuses);
                // We probably want to have the registry updates separate, so
                // that we don't update every 5 seconds
                let _ = rs.update_node_details(&node_providers).await;
                for notification in notifications {
                    let node = rs.node(notification.node_id).await;

                    // NOTE: This might break and not kill the complete program.
                    // What happens when we have an exception in an other
                    // process that gets killed ?
                    config
                        .notification_sender
                        .send(Notification {
                            node_provider: Some(node.operator.provider),
                            ..notification.clone()
                        })
                        .expect("Could not send notification. The notification sender is probably dead, exitting...");
                }
                nodes_status = new_nodes_status;
            }
            Err(e) => {
                // TODO if we cannot get the nodes, this will kill the loop.
                // Make sure we are resilient there
                config.cancellation_token.cancel();
                config.service_health.set_health_check_loop_readiness(true);
                error!(
                    message = "Issue while getting the nodes statuses",
                    error = e.to_string()
                );
                break;
            }
        }
        sleep(time::Duration::from_secs(5)).await;
    }
}
