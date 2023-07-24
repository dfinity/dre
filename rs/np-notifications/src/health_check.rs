use actix_web::rt::time::sleep;
use core::time;
use std::sync::mpsc::Sender;

use ic_management_backend::health::HealthClient;
use slog::Logger;
use tokio_util::sync::CancellationToken;

use crate::{nodes_status::NodesStatus, notification::Notification};

pub struct HealthCheckLoopConfig {
    pub logger: Logger,
    pub notification_sender: Sender<Notification>,
    pub cancellation_token: CancellationToken,
}

pub async fn start_health_check_loop(config: HealthCheckLoopConfig) {
    debug!(config.logger, "Starting health check loop");
    let hc = HealthClient::new(ic_management_types::Network::Mainnet);
    let mut nodes_status = NodesStatus::from(hc.nodes().await.unwrap());

    loop {
        if config.cancellation_token.is_cancelled() {
            break;
        }
        let (new_nodes_status, notifications) = nodes_status.updated_from_map(hc.nodes().await.unwrap());
        for notification in notifications {
            config
                .notification_sender
                .send(notification.clone())
                .expect("Could not send notification. The notification sender is probably dead, exitting...");
        }
        nodes_status = new_nodes_status;
        sleep(time::Duration::from_secs(5)).await;
    }
}
