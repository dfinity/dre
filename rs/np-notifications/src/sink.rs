use std::cell::RefCell;
use std::rc::Rc;

use anyhow::anyhow;
use anyhow::Result;
use reqwest::StatusCode;
use tracing::{debug, error, info};

use crate::notification::Notification;

#[derive(Debug)]
pub enum Sink {
    Log(LogSink),
    #[allow(unused)]
    Webhook(WebhookSink),
    #[allow(unused)]
    Test(Rc<TestSink>),
}

impl Sink {
    pub async fn send(&self, notification: Notification) -> Result<()> {
        match self {
            Sink::Log(sink) => sink.send(notification),
            Sink::Webhook(sink) => sink.send(notification).await,
            Sink::Test(sink) => {
                sink.send(notification);
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct LogSink {}

impl LogSink {
    fn send(&self, notification: Notification) -> Result<()> {
        info!(sink = "log", notification = serde_json::to_string(&notification)?);
        Ok(())
    }
}

#[derive(Debug)]
pub struct WebhookSink {
    pub url: url::Url,
    pub _auth: Option<(String, String)>,
}

impl WebhookSink {
    async fn send(&self, notification: Notification) -> Result<()> {
        debug!(
            message = "Sending notification",
            url = &self.url.to_string(),
            notification = notification.to_string(),
        );
        let client = reqwest::Client::new();
        let response = client.post(&self.url.to_string()).json(&notification).send().await.map_err(|e| {
            error!(
                message = "Error while sending the notification",
                notification = notification.to_string(),
                error = e.to_string(),
            );
            e
        })?;
        match response.status() {
            StatusCode::OK => Ok(()),
            _ => {
                error!(
                    message = "Error while sending the notification",
                    notification = notification.to_string(),
                    status = response.status().to_string(),
                    response = response.text().await?,
                );
                Err(anyhow!("Failed to send notification"))
            }
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

#[cfg(test)]
mod test {
    use crate::{
        notification::Notification,
        sink::{LogSink, Sink},
    };

    use super::WebhookSink;
    use httptest::{all_of, matchers::request, responders::status_code, Expectation};
    use test_log::test;

    #[actix_web::test]
    async fn webhook_sends_requests() {
        let notification = Notification::new_test(0);
        let server = httptest::Server::run();

        server.expect(
            Expectation::matching(all_of![
                request::method_path("POST", "/success"),
                request::body(serde_json::to_string(&notification).unwrap())
            ])
            .respond_with(status_code(200)),
        );
        let wh = WebhookSink {
            url: url::Url::parse(&server.url("/success").to_string()).unwrap(),
            auth: None,
        };
        let result = wh.send(notification.clone()).await;
        assert!(result.is_ok());

        server.expect(
            Expectation::matching(all_of![
                request::method_path("POST", "/failure"),
                request::body(serde_json::to_string(&notification).unwrap())
            ])
            .respond_with(status_code(500)),
        );
        let wh = WebhookSink {
            url: url::Url::parse(&server.url("/failure").to_string()).unwrap(),
            auth: None,
        };
        let result = wh.send(notification).await;
        assert!(result.is_err());
    }

    #[test(actix_web::test)]
    async fn log_sink_display() {
        // test mostly used to see our log lines.
        // To display the log line, set RUST_LOG=info and make the test fail
        let n = Notification::new_test(0);
        let log_sink = Sink::Log(LogSink {});
        let _ = log_sink.send(n.clone()).await;
    }
}
