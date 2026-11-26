"""
Announces the release notes of a new version on Open Messaging.

Open Messaging accepts Slack's incoming-webhook format through its gateway
canister, but it renders only an attachment's title, title link, colour bar
and mrkdwn text.  A top-level ``text`` shows its ``<url|label>`` links raw,
drops ``<!subteam^…>`` mentions and prints emoji codes as text, so the
announcement is a single attachment: the version links its tag in the title
and the text carries the Google Doc link.

The webhook address holds its secret, so nothing that could carry it is
logged or raised without being redacted first.
"""

import logging
import os
import time
import typing
import urllib.parse

import requests
from dotenv import load_dotenv

from const import GUESTOS, OsKind

LOGGER = logging.getLogger(__name__)

OPEN_MESSAGING_WEBHOOK_URL_ENV = "OPEN_MESSAGING_WEBHOOK_URL"

# How long one post may take.
TIMEOUT_SECONDS = 30
# Retries after a failure to connect.
MAX_RETRIES = 2
RETRY_DELAY_SECONDS = 2.0
# How much of an error body is kept in the error that is raised.
MAX_ERROR_BODY = 200


class ReleaseAnnouncerProtocol(typing.Protocol):
    def announce_release(
        self,
        webhook: str,
        version_name: str,
        google_doc_url: str,
        os_kind: OsKind,
    ) -> None: ...


class OpenMessagingAnnouncer(ReleaseAnnouncerProtocol):
    def announce_release(
        self,
        webhook: str,
        version_name: str,
        google_doc_url: str,
        os_kind: OsKind,
    ) -> None:
        announce_release(webhook, version_name, google_doc_url, os_kind)


class OpenMessagingError(Exception):
    """A post to Open Messaging failed.  Its message is already redacted."""


def webhook_from_env() -> str | None:
    """The Open Messaging webhook, or None when it is unset or empty."""
    return os.environ.get(OPEN_MESSAGING_WEBHOOK_URL_ENV) or None


def payload(
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


def post(webhook_url: str, body: dict[str, typing.Any]) -> None:
    """
    Post a payload to an Open Messaging webhook.

    The gateway answers a good post with a plain ``ok``, so a 2xx alone is
    not taken as success.  A failure to connect is retried; nothing else
    is, so that a post that may have arrived is not posted twice.
    """
    attempt = 0
    while True:
        try:
            response = requests.post(webhook_url, json=body, timeout=TIMEOUT_SECONDS)
            break
        except requests.RequestException as e:
            # Formatted with the address, so redacted, and raised without
            # its cause, which would print the address with the traceback.
            message = redact(webhook_url, str(e))
            if isinstance(e, requests.ConnectionError) and attempt < MAX_RETRIES:
                attempt += 1
                LOGGER.warning(
                    "Could not reach the Open Messaging webhook, retrying (%s/%s): %s",
                    attempt,
                    MAX_RETRIES,
                    message,
                )
                time.sleep(RETRY_DELAY_SECONDS * attempt)
                continue
            raise OpenMessagingError(
                f"could not reach the Open Messaging webhook: {message}"
            ) from None

    answer = response.text
    if not (200 <= response.status_code < 300) or answer.strip() != "ok":
        # Redacted before it is cut short, so that a cut cannot leave part
        # of the secret behind.
        answer = redact(webhook_url, answer)[:MAX_ERROR_BODY]
        raise OpenMessagingError(
            f"Open Messaging webhook answered {response.status_code}: {answer}"
        )


def announce_release(
    webhook_url: str,
    version_name: str,
    google_doc_url: str,
    os_kind: OsKind,
) -> None:
    post(webhook_url, payload(version_name, google_doc_url, os_kind))
    LOGGER.info("Announced %s %s on Open Messaging", os_kind, version_name)


def main() -> None:
    """Post a sample announcement to the webhook in OPEN_MESSAGING_WEBHOOK_URL."""
    load_dotenv()

    webhook = webhook_from_env()
    if webhook is None:
        raise SystemExit(f"{OPEN_MESSAGING_WEBHOOK_URL_ENV} is not set")
    announce_release(
        webhook,
        "release-2024-03-06_23-01-base",
        "https://docs.google.com/document/d/1gCPmYxoq9_IccdChRzjoblAggTOdZ_IfTMukRbODO1I/edit#heading=h.7dcpz3fj7xrh",
        GUESTOS,
    )


if __name__ == "__main__":
    main()
