use core::time;
use std::{
    fmt::{self, Display},
    sync::mpsc::Receiver,
};

use actix_web::rt::time::sleep;
use ic_management_types::{Provider, Status};
use ic_types::PrincipalId;
use slog::Logger;
use tokio_util::sync::CancellationToken;

use crate::{router::Router, sink::Sink};

pub struct NotificationSenderLoopConfig {
    pub logger: Logger,
    pub notification_receiver: Receiver<Notification>,
    pub cancellation_token: CancellationToken,
    pub router: Router,
}

pub async fn start_notification_sender_loop(config: NotificationSenderLoopConfig, sinks: Vec<Sink>) {
    debug!(config.logger, "Starting notification sender loop");
    loop {
        if config.cancellation_token.is_cancelled() {
            break;
        }
        while let Ok(notification) = config.notification_receiver.try_recv() {
            for sink in sinks.iter() {
                let _ = sink.send(notification.clone()).await;
            }
            let _ = config.router.route(notification).await;
        }
        sleep(time::Duration::from_secs(1)).await;
    }
}

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Clone)]
pub struct Notification {
    pub node_id: PrincipalId,
    pub node_provider: Option<Provider>,
    pub status_change: (Status, Status),
}

impl Display for Notification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Provider {} \nNode {} changed status \n\t{} -> {}",
            // TODO Manage no Node provider name better
            self.node_provider.clone().unwrap().name.unwrap(),
            self.node_id,
            self.status_change.0,
            self.status_change.1
        )
    }
}
