"""
Posts trusted-neurons alerts to an Open Messaging channel.

Open Messaging accepts Slack's incoming-webhook format through its gateway
canister, but it renders only an attachment's title, title link, colour bar
and mrkdwn text: buttons are dropped and :emoji: codes show as plain text.
So instead of the Slack app's Vote and Silence buttons, each message carries
both as links. Silence opens Alertmanager's new-silence form, pre-filled with
the proposal's matcher.
"""

import logging
from urllib.parse import quote
from urllib.parse import urlsplit

import aiohttp

# The most urgent alert gets a red bar, the reminders an amber one.
URGENT_ALERTS = {"ExpiresInLessThan2Hours"}


class OpenMessagingError(Exception):
    """
    The webhook did not accept a post.

    The message never contains the webhook address, which holds the secret.
    """


class OpenMessaging:
    """Client for one Open Messaging incoming webhook."""

    def __init__(self, webhook_url: str, alertmanager_url: str, timeout_seconds: float = 30):
        """Post to webhook_url; link silences to the Alertmanager at alertmanager_url."""
        self.webhook_url = webhook_url
        self.alertmanager_url = alertmanager_url.rstrip("/")
        self.timeout = aiohttp.ClientTimeout(total=timeout_seconds)

    def redact(self, text: str) -> str:
        """
        Remove the webhook's address, path and secret from text.

        A gateway or proxy may echo the request path in an error body, and
        the path ends with the secret.
        """
        path = urlsplit(self.webhook_url).path.rstrip("/")
        secret = path.rsplit("/", 1)[-1]
        for value in (self.webhook_url, path, secret):
            if len(value) >= 8:
                text = text.replace(value, "<redacted>")
        return text

    def silence_url(self, proposal_id: str) -> str:
        """Alertmanager's new-silence form, pre-filled for the proposal."""
        matcher = quote(f'{{proposal_id="{proposal_id}"}}', safe="")
        return f"{self.alertmanager_url}/#/silences/new?filter={matcher}"

    def format_alert(self, alertname: str, proposal_id: str, proposal_topic: str, proposal_type: str) -> dict:
        """Build the Slack-format payload for one alert."""
        title = f"Proposal {proposal_id}: {alertname}"
        vote_url = f"https://nns.icp.net/voting/proposals/{proposal_id}"
        text = (
            f"*Topic:* {proposal_topic}\n"
            f"*Type:* {proposal_type}\n"
            f"<{vote_url}|Vote> · <{self.silence_url(proposal_id)}|Silence>"
        )
        return {
            "attachments": [
                {
                    "title": title,
                    "title_link": f"https://dashboard.internetcomputer.org/proposal/{proposal_id}",
                    "text": text,
                    "fallback": title,
                    "color": "danger" if alertname in URGENT_ALERTS else "warning",
                    "mrkdwn_in": ["text", "fallback"],
                }
            ]
        }

    async def send_alert(self, alertname: str, proposal_id: str, proposal_topic: str, proposal_type: str) -> None:
        """Post one alert. The gateway answers a good post with a plain "ok"."""
        payload = self.format_alert(alertname, proposal_id, proposal_topic, proposal_type)
        try:
            async with aiohttp.ClientSession(timeout=self.timeout) as session:
                async with session.post(self.webhook_url, json=payload) as response:
                    status = response.status
                    body = await response.text()
        except (aiohttp.ClientError, TimeoutError) as e:
            raise OpenMessagingError(f"could not reach the Open Messaging webhook: {type(e).__name__}") from None

        if status != 200 or body.strip() != "ok":
            # Redact before truncating, so a cut can't leave part of the secret.
            raise OpenMessagingError(f"Open Messaging webhook answered {status}: {self.redact(body)[:200]}")
        logging.info("Posted proposal %s (%s) to Open Messaging", proposal_id, alertname)
