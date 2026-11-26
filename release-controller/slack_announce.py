"""
Announces the release notes of a new version on Slack and on Open Messaging.

Open Messaging accepts Slack's incoming-webhook format through its gateway
canister, but it renders only an attachment's title, title link, colour bar
and mrkdwn text.  A top-level ``text`` shows its ``<url|label>`` links raw,
drops ``<!subteam^…>`` mentions and prints emoji codes as text, so the same
announcement is rendered there as a single attachment instead: the version
links its tag in the title, the text carries the Google Doc link, and the
team mentions and the emoji are dropped, since there is nobody to ping.

The webhook address holds its secret, so nothing that could carry it is
logged or raised without being redacted first.
"""

import logging
import os
import time
import urllib.parse

import requests
from dotenv import load_dotenv
from slack_sdk.http_retry.handler import RetryHandler
from slack_sdk.webhook import WebhookClient
import typing

from const import OsKind, GUESTOS
from release_notes_composer import RELEASE_NOTES_REVIEWERS

LOGGER = logging.getLogger(__name__)

SLACK_WEBHOOK_URL_ENV = "SLACK_WEBHOOK_URL"
OPEN_MESSAGING_WEBHOOK_URL_ENV = "OPEN_MESSAGING_WEBHOOK_URL"

# How long one post to Open Messaging may take.
OPEN_MESSAGING_TIMEOUT_SECONDS = 30
# Retries after a failure to connect, as many as the Slack client makes.
OPEN_MESSAGING_MAX_RETRIES = 2
OPEN_MESSAGING_RETRY_DELAY_SECONDS = 2.0
# How much of an error body is kept in the error that is raised.
MAX_ERROR_BODY = 200


class SlackAnnouncerProtocol(typing.Protocol):
    def announce_release(
        self,
        webhook: str,
        version_name: str,
        google_doc_url: str,
        os_kind: OsKind,
    ) -> None: ...

    def announce_release_on_open_messaging(
        self,
        webhook: str,
        version_name: str,
        google_doc_url: str,
        os_kind: OsKind,
    ) -> None: ...


class SlackAnnouncer(SlackAnnouncerProtocol):
    def announce_release(
        self,
        webhook: str,
        version_name: str,
        google_doc_url: str,
        os_kind: OsKind,
    ) -> None:
        announce_release_on_slack(webhook, version_name, google_doc_url, os_kind)

    def announce_release_on_open_messaging(
        self,
        webhook: str,
        version_name: str,
        google_doc_url: str,
        os_kind: OsKind,
    ) -> None:
        announce_release_on_open_messaging(
            webhook, version_name, google_doc_url, os_kind
        )


class OpenMessagingError(Exception):
    """A post to Open Messaging failed.  Its message is already redacted."""


def open_messaging_webhook_from_env() -> str | None:
    """The Open Messaging webhook, or None when it is unset or empty."""
    return os.environ.get(OPEN_MESSAGING_WEBHOOK_URL_ENV) or None


def announce_release(
    announcer: SlackAnnouncerProtocol,
    slack_webhook: str | None,
    open_messaging_webhook: str | None,
    version_name: str,
    google_doc_url: str,
    os_kind: OsKind,
) -> None:
    """
    Announce the release notes on every destination that has a webhook.

    The destinations are independent: each is tried even if the other
    failed.  Once both have been tried, a failure is raised, so that the
    phase is marked as failed as it was when Slack was the only one.
    """
    errors: list[Exception] = []
    if slack_webhook is not None:
        try:
            announcer.announce_release(
                webhook=slack_webhook,
                version_name=version_name,
                google_doc_url=google_doc_url,
                os_kind=os_kind,
            )
        except Exception as e:
            if open_messaging_webhook is None:
                # The only destination: fail exactly as before.
                raise
            LOGGER.error("Could not announce %s on Slack: %s", version_name, e)
            errors.append(e)
    if open_messaging_webhook is not None:
        try:
            announcer.announce_release_on_open_messaging(
                webhook=open_messaging_webhook,
                version_name=version_name,
                google_doc_url=google_doc_url,
                os_kind=os_kind,
            )
        except Exception as e:
            # Logged with the message only: the cause of an error raised by
            # a client may hold the address, and its secret with it.
            message = redact(open_messaging_webhook, str(e))
            LOGGER.error(
                "Could not announce %s on Open Messaging: %s", version_name, message
            )
            errors.append(OpenMessagingError(message))
    if len(errors) == 1:
        raise errors[0]
    if errors:
        raise ExceptionGroup(
            f"Could not announce {version_name} on Slack nor on Open Messaging",
            errors,
        )


def announce_release_on_slack(
    slack_url: str,
    version_name: str,
    google_doc_url: str,
    os_kind: OsKind,
) -> None:
    slack = WebhookClient(
        url=slack_url, retry_handlers=[RetryHandler(max_retry_count=2)]
    )
    notify_line = " ".join(
        [
            f"<!subteam^{t.slack_id}>"
            for t in RELEASE_NOTES_REVIEWERS[os_kind]
            if t.send_announcement
        ]
    )
    if not notify_line:
        # No teams elected to be notified about this wondrous event.
        # Revert to Farnsworth mode.
        notify_line = "everyone"
    slack.send(
        text=f"""\
Good news {notify_line}!
Here are the {os_kind} release notes for the rollout of <https://github.com/dfinity/ic/tree/{version_name}|`{version_name}`> <{google_doc_url}|Release Notes> :railway_car:
""",
    )


def open_messaging_payload(
    version_name: str,
    google_doc_url: str,
    os_kind: OsKind,
) -> dict[str, typing.Any]:
    """The announcement as Open Messaging renders it: a single attachment."""
    title = f"{os_kind} release notes: {version_name}"
    text = (
        f"Good news! Here are the {os_kind} release notes for the rollout of"
        f" `{version_name}`: <{google_doc_url}|Release Notes>"
    )
    return {
        "attachments": [
            {
                "title": title,
                "title_link": f"https://github.com/dfinity/ic/tree/{version_name}",
                "text": text,
                "fallback": title,
                "color": "good",
                "mrkdwn_in": ["text", "fallback"],
            }
        ]
    }


def redact(webhook_url: str, text: str) -> str:
    """
    Remove the webhook's address, path and secret from a text.

    A gateway or proxy may echo the request path in an error body, and
    ``requests`` puts the address in the errors it formats, so anything
    derived from a failed post is passed through here before it is logged.
    """
    path = urllib.parse.urlsplit(webhook_url).path.rstrip("/")
    secret = path.rsplit("/", 1)[-1]
    for value in (webhook_url, path, secret):
        # Short values are too generic to be the secret, and replacing them
        # would mangle unrelated text.
        if len(value) >= 8:
            text = text.replace(value, "<redacted>")
    return text


def post_to_open_messaging(webhook_url: str, payload: dict[str, typing.Any]) -> None:
    """
    Post a payload to an Open Messaging webhook.

    The gateway answers a good post with a plain ``ok``, so a 2xx alone is
    not taken as success.  A failure to connect is retried, as the Slack
    client does; nothing else is, so that a post that may have arrived is
    not posted twice.
    """
    attempt = 0
    while True:
        try:
            response = requests.post(
                webhook_url, json=payload, timeout=OPEN_MESSAGING_TIMEOUT_SECONDS
            )
            break
        except requests.RequestException as e:
            # Formatted with the address, so redacted, and raised without
            # its cause, which would print the address with the traceback.
            message = redact(webhook_url, str(e))
            if (
                isinstance(e, requests.ConnectionError)
                and attempt < OPEN_MESSAGING_MAX_RETRIES
            ):
                attempt += 1
                LOGGER.warning(
                    "Could not reach the Open Messaging webhook, retrying (%s/%s): %s",
                    attempt,
                    OPEN_MESSAGING_MAX_RETRIES,
                    message,
                )
                time.sleep(OPEN_MESSAGING_RETRY_DELAY_SECONDS * attempt)
                continue
            raise OpenMessagingError(
                f"could not reach the Open Messaging webhook: {message}"
            ) from None

    body = response.text
    if not (200 <= response.status_code < 300) or body.strip() != "ok":
        # Redacted before it is cut short, so that a cut cannot leave part
        # of the secret behind.
        body = redact(webhook_url, body)[:MAX_ERROR_BODY]
        raise OpenMessagingError(
            f"Open Messaging webhook answered {response.status_code}: {body}"
        )


def announce_release_on_open_messaging(
    webhook_url: str,
    version_name: str,
    google_doc_url: str,
    os_kind: OsKind,
) -> None:
    post_to_open_messaging(
        webhook_url, open_messaging_payload(version_name, google_doc_url, os_kind)
    )
    LOGGER.info("Announced %s %s on Open Messaging", os_kind, version_name)


def main() -> None:
    load_dotenv()

    announce_release_on_slack(
        os.environ["SLACK_WEBHOOK_URL"],
        "release-2024-03-06_23-01-base",
        "https://docs.google.com/document/d/1gCPmYxoq9_IccdChRzjoblAggTOdZ_IfTMukRbODO1I/edit#heading=h.7dcpz3fj7xrh",
        GUESTOS,
    )
    announce_release_on_slack(
        os.environ["SLACK_WEBHOOK_URL"],
        "release-2024-03-06_23-01-p2p",
        "https://docs.google.com/document/d/1gCPmYxoq9_IccdChRzjoblAggTOdZ_IfTMukRbODO1I/edit#heading=h.7dcpz3fj7xrh",
        GUESTOS,
    )


if __name__ == "__main__":
    main()
