use core::time;
use std::{
    cell::RefCell,
    fmt::{self, Display},
    sync::mpsc::Receiver,
};

use actix_web::rt::time::sleep;
use ic_management_types::{Provider, Status};
use ic_types::PrincipalId;
use slog::Logger;
use tokio_util::sync::CancellationToken;

use crate::matrix;

pub struct NotificationSenderLoopConfig {
    pub logger: Logger,
    pub notification_receiver: Receiver<Notification>,
    pub cancellation_token: CancellationToken,
}

pub async fn start_notification_sender_loop(config: NotificationSenderLoopConfig, sinks: Vec<Sink>) {
    debug!(config.logger, "Starting notification sender loop");
    loop {
        if config.cancellation_token.is_cancelled() {
            break;
        }
        while let Ok(notification) = config.notification_receiver.try_recv() {
            let notif = notification.clone();
            for sink in sinks.iter() {
                let _ = sink.send(notif.clone()).await;
            }
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
            "Provider {:?} \nNode {} changed status \n\t{} -> {}",
            &self.node_provider, self.node_id, self.status_change.0, self.status_change.1
        )
    }
}

#[derive(Debug)]
pub enum SinkError {
    PublicationError,
}

pub enum Sink {
    Log(LogSink),
    Matrix(MatrixSink),
    #[allow(unused)]
    Test(TestSink),
}

impl Sink {
    async fn send(&self, notification: Notification) -> Result<(), SinkError> {
        match self {
            Sink::Log(sink) => sink.send(notification),
            Sink::Matrix(sink) => sink.send(notification).await,
            Sink::Test(sink) => {
                sink.send(notification);
                Ok(())
            }
        }
    }
}

#[derive(Clone)]
pub struct LogSink {
    pub logger: Logger,
}

impl LogSink {
    fn send(&self, notification: Notification) -> Result<(), SinkError> {
        info!(self.logger, ""; "sink" => "log", "notification" => ?notification);
        Ok(())
    }
}

pub struct MatrixSink {
    pub logger: Logger,
    pub matrix_client: matrix::Client,
}

impl MatrixSink {
    async fn send(&self, notification: Notification) -> Result<(), SinkError> {
        self.matrix_client
            .send_message_to_room("!jeoHAONXXskUWAPpKH:matrix.org".to_string(), notification.to_string())
            .await
            .map_err(|_| SinkError::PublicationError)
            .map(|_| ())
    }
}

pub struct TestSink {
    pub notifications: RefCell<Vec<Notification>>,
}

impl TestSink {
    fn send(&self, notification: Notification) {
        self.notifications.borrow_mut().push(notification)
    }

    #[allow(unused)]
    pub fn new() -> Self {
        Self {
            notifications: RefCell::new(vec![]),
        }
    }
}
