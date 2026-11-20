//! Chooses where a group of proposals is announced, and posts it there.
//!
//! Each of the two proposal streams, internal and external, goes either to
//! Slack, as it always has, or to an Open Messaging channel once that stream
//! has a webhook set. Open Messaging takes Slack's incoming-webhook format,
//! but renders only an attachment's fields, so the payload is built per
//! destination; see [`crate::open_messaging`].

use anyhow::{Result, anyhow};
use log::info;
use serde_json::json;

use crate::open_messaging;
use crate::slack::SlackMessage;

pub const SLACK_URL_ENV: &str = "SLACK_URL";

/// Where one group of proposals is announced.
///
/// Deliberately without a `Debug` implementation: an Open Messaging address
/// ends with the webhook's secret, and must not reach the log through a
/// derived formatter.
#[derive(Clone, PartialEq, Eq)]
pub enum Destination {
    /// A channel of the Slack workspace that `SLACK_URL` posts to.
    Slack { channel: String },
    /// One Open Messaging channel, named by its own incoming webhook.
    OpenMessaging { webhook_url: String },
}

/// Removes a webhook's address, path and secret from `text`.
///
/// A gateway or proxy may echo the request path in an error body, and
/// `reqwest` puts the whole address in the errors it formats, so anything
/// derived from a failed post is passed through here before it is logged.
pub fn redact(webhook_url: &str, text: &str) -> String {
    let mut text = text.to_string();
    let path = reqwest::Url::parse(webhook_url)
        .map(|url| url.path().trim_end_matches('/').to_string())
        .unwrap_or_default();
    let secret = path.rsplit('/').next().unwrap_or_default().to_string();
    for value in [webhook_url, path.as_str(), secret.as_str()] {
        // Short values are too generic to be the secret, and replacing them
        // would mangle unrelated text.
        if value.len() >= 8 {
            text = text.replace(value, "<redacted>");
        }
    }
    text
}

/// Posts proposal announcements to Slack, to Open Messaging, or to both.
pub struct Notifier {
    client: reqwest::Client,
    /// Absent when `SLACK_URL` is unset, which is allowed once both streams
    /// are on Open Messaging.
    slack_url: Option<String>,
}

impl Notifier {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            slack_url: std::env::var(SLACK_URL_ENV).ok().filter(|url| !url.is_empty()),
        }
    }

    pub async fn send(&self, message: &SlackMessage) -> Result<()> {
        match &message.destination {
            Destination::Slack { channel } => self.send_to_slack(message, channel).await,
            Destination::OpenMessaging { webhook_url } => open_messaging::send(&self.client, webhook_url, message).await,
        }
    }

    async fn send_to_slack(&self, message: &SlackMessage, channel: &str) -> Result<()> {
        let url = self
            .slack_url
            .as_ref()
            .ok_or_else(|| anyhow!("{} must be set to post to the Slack channel {}", SLACK_URL_ENV, channel))?;

        let mut payload = message.render_payload();
        payload["channel"] = json!(channel);
        info!("Sending Slack payload: {}", serde_json::to_string(&payload).unwrap_or_default());

        // `SLACK_URL` is itself a secret, so a transport or status error,
        // which `reqwest` formats with the address, is redacted before it is
        // returned to the caller that logs it.
        self.client
            .post(url)
            .json(&payload)
            .send()
            .await
            .and_then(|response| response.error_for_status())
            .map_err(|e| anyhow!("{}", redact(url, &e.to_string())))?;

        info!("Posted {} proposal(s) to the Slack channel {}", message.proposals.len(), channel);
        Ok(())
    }
}

impl Default for Notifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WEBHOOK: &str = "https://gateway.example/api/hooks/abc123/s3cr3t-value";

    #[test]
    fn redact_removes_the_address_the_path_and_the_secret() {
        // A gateway may echo any of the three back in an error body.
        assert_eq!(redact(WEBHOOK, &format!("posting to {} failed", WEBHOOK)), "posting to <redacted> failed");
        assert_eq!(redact(WEBHOOK, "no route for /api/hooks/abc123/s3cr3t-value"), "no route for <redacted>");
        assert_eq!(redact(WEBHOOK, "unknown hook s3cr3t-value"), "unknown hook <redacted>");
    }

    #[test]
    fn redact_leaves_unrelated_text_alone() {
        assert_eq!(redact(WEBHOOK, "502 Bad Gateway"), "502 Bad Gateway");
    }

    #[test]
    fn redact_survives_an_unparsable_address() {
        // Nothing to strip, but it must not panic or return the address.
        assert_eq!(redact("not a url", "502 Bad Gateway"), "502 Bad Gateway");
    }
}
