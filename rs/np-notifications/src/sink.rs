use std::{cell::RefCell, sync::Arc};

use slog::Logger;

use crate::notification::Notification;

#[derive(Debug)]
pub enum SinkError {
    PublicationError,
}

#[derive(Debug)]
pub enum Sink {
    Log(LogSink),
    #[allow(unused)]
    Webhook(WebhookSink),
    #[allow(unused)]
    Test(Arc<TestSink>),
}

impl Sink {
    pub async fn send(&self, notification: Notification) -> Result<(), SinkError> {
        match self {
            Sink::Log(sink) => sink.send(notification),
            Sink::Webhook(sink) => sink.send(notification),
            Sink::Test(sink) => {
                sink.send(notification);
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct LogSink {
    pub logger: Logger,
}

impl LogSink {
    fn send(&self, notification: Notification) -> Result<(), SinkError> {
        info!(self.logger, ""; "sink" => "log", "notification" => ?notification);
        Ok(())
    }
}

#[derive(Debug)]
pub struct WebhookSink {
    pub url: url::Url,
    pub auth: Option<(String, String)>,
}

impl WebhookSink {
    fn send(&self, notification: Notification) -> Result<(), SinkError> {
        // Mock implementation, used to satisfy unused other structures
        dbg!("Sending {} to webhook", &notification);
        if notification.node_id.to_string().starts_with('a') {
            Err(SinkError::PublicationError)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug)]
pub struct TestSink {
    pub notifications: RefCell<Vec<Notification>>,
}

impl TestSink {
    fn send(&self, notification: Notification) {
        self.notifications.borrow_mut().push(notification)
    }

    #[cfg(test)]
    pub fn new() -> Self {
        Self {
            notifications: RefCell::new(vec![]),
        }
    }

    #[cfg(test)]
    pub fn notifications(&self) -> Vec<Notification> {
        self.notifications.borrow().clone()
    }
}
