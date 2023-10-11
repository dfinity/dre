use core::time;
use std::{
    fmt::{self, Display},
    sync::mpsc::Receiver,
};

use actix_web::rt::time::sleep;
use ic_management_types::{Provider, Status};
use ic_types::PrincipalId;
use serde::ser::{SerializeStruct, Serializer};
use serde::Serialize;
use tokio_util::sync::CancellationToken;
use tracing::debug;

use crate::{router::Router, sink::Sink};

#[derive(Debug)]
pub struct NotificationSenderLoopConfig {
    pub notification_receiver: Receiver<Notification>,
    pub cancellation_token: CancellationToken,
    pub router: Router,
}

#[tracing::instrument]
pub async fn start_notification_sender_loop(config: NotificationSenderLoopConfig, sinks: Vec<Sink>) {
    debug!("Starting notification sender loop");
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

impl Serialize for Notification {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Notification", 1)?;
        state.serialize_field("node_id", &self.node_id.to_string())?;
        if let Some(provider) = &self.node_provider {
            state.serialize_field("node_provider_id", &provider.principal.to_string())?;
        }
        state.serialize_field("status_change", &self.status_change)?;
        state.end()
    }
}

#[cfg(test)]
impl Notification {
    pub fn new_test(id: u64) -> Self {
        Self {
            node_id: PrincipalId::new_node_test_id(id),
            node_provider: Some(Provider {
                principal: PrincipalId::new_user_test_id(id),
                name: Some("test".into()),
                website: None,
            }),
            status_change: (Status::Healthy, Status::Degraded),
        }
    }
}
