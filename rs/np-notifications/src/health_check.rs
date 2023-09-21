use actix_web::rt::time::sleep;
use core::time;
use std::sync::mpsc::Sender;

use ic_management_backend::{health::HealthClient, registry::RegistryState};
use slog::Logger;
use tokio_util::sync::CancellationToken;

use crate::{nodes_status::NodesStatus, notification::Notification};

pub struct HealthCheckLoopConfig {
    pub logger: Logger,
    pub notification_sender: Sender<Notification>,
    pub cancellation_token: CancellationToken,
    pub registry_state: RegistryState,
}

pub async fn start_health_check_loop(config: HealthCheckLoopConfig) {
    let log = config.logger;
    debug!(log, "Starting health check loop");
    let hc = HealthClient::new(ic_management_types::Network::Mainnet);
    let mut nodes_status = NodesStatus::from(hc.nodes().await.unwrap());

    let mut rs = config.registry_state;
    loop {
        if config.cancellation_token.is_cancelled() {
            break;
        }
        match hc.nodes().await {
            Ok(new_statuses) => {
                // Probably need to change the way we create the notifications there to
                // include the fetching from the registry
                let (new_nodes_status, notifications) = nodes_status.updated_from_map(new_statuses);
                let _ = rs.update(vec![], vec![]).await;
                for notification in notifications {
                    let node = rs.node(notification.node_id).await;

                    // NOTE: This might break and not kill the complete program.
                    // What happens when we have an exception in an other process
                    // that gets killed ?
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
                config.cancellation_token.cancel();
                error!(log, "Issue while getting the nodes statuses"; "error" => e.to_string());
                break;
            }
        }
        sleep(time::Duration::from_secs(5)).await;
    }
}
