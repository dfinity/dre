//! Posts proposal announcements to an Open Messaging channel.
//!
//! Open Messaging accepts Slack's incoming-webhook format through its gateway
//! canister, but it renders only an attachment's title, title link, colour bar
//! and mrkdwn text. Block Kit blocks, which is what the Slack announcement is
//! built from, and Slack's group and user mentions do not survive the trip, so
//! the same proposals are rendered here as a single attachment: the mentions
//! are dropped, since there is no group to ping, and the proposer is named by
//! neuron id rather than by a `<@U…>` handle that would show up raw.
//!
//! The webhook names the channel, so, unlike the Slack hook, this one carries
//! no channel field: one webhook, one channel.

use anyhow::{Result, bail};
use log::info;
use serde_json::{Value, json};

use crate::notify::redact;
use crate::slack::{SlackMessage, proposal_link_markdown, proposal_url};

/// How much of an error body is kept in the message that is logged.
const MAX_ERROR_BODY: usize = 200;

/// Builds the payload for one group of proposals.
pub fn render_payload(message: &SlackMessage) -> Value {
    let failed = message.all_failed();
    let count = message.proposals.len();
    let title = if failed {
        format!("{} proposal(s) failed", count)
    } else {
        format!("Please review {} proposal(s)", count)
    };

    let links = message
        .proposals
        .iter()
        .map(|p| proposal_link_markdown(p.id.expect("no proposal id")))
        .collect::<Vec<_>>()
        .join(" · ");

    let text = format!(
        "{}\n\n*Proposals:* {}\n*Proposed by:* {}",
        message.motivation,
        links,
        message.proposer_name()
    );

    json!({
        "attachments": [
            {
                "title": title,
                "title_link": message
                    .proposals
                    .first()
                    .map(|p| proposal_url(p.id.expect("no proposal id")))
                    .unwrap_or_default(),
                "text": text,
                "fallback": title,
                "color": if failed { "danger" } else { "warning" },
                "mrkdwn_in": ["text", "fallback"],
            }
        ]
    })
}

/// Posts one group of proposals. The gateway answers a good post with a plain
/// `ok`, so a 200 alone is not taken as success.
pub async fn send(client: &reqwest::Client, webhook_url: &str, message: &SlackMessage) -> Result<()> {
    let payload = render_payload(message);
    info!("Sending Open Messaging payload: {}", serde_json::to_string(&payload).unwrap_or_default());

    // The address holds the secret, and `reqwest` formats its errors with the
    // address, so every error built from here is redacted first.
    let response = client
        .post(webhook_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("could not reach the Open Messaging webhook: {}", redact(webhook_url, &e.to_string())))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() || body.trim() != "ok" {
        // Redacted before it is truncated, so that a cut cannot leave part of
        // the secret behind.
        let body: String = redact(webhook_url, &body).chars().take(MAX_ERROR_BODY).collect();
        bail!("Open Messaging webhook answered {}: {}", status, body);
    }

    info!("Posted {} proposal(s) to Open Messaging", message.proposals.len());
    Ok(())
}
