import os

from dotenv import load_dotenv
from slack_sdk.http_retry.handler import RetryHandler
from slack_sdk.webhook import WebhookClient
import typing

from const import OsKind, GUESTOS
from release_notes import RELEASE_NOTES_REVIEWERS


class SlackAnnouncerProtocol(typing.Protocol):
    def announce_release(
        self,
        webhook: str,
        version_name: str,
        google_doc_url: str,
        tag_all_teams: bool,
        os_kind: OsKind,
    ) -> None: ...


class SlackAnnouncer(SlackAnnouncerProtocol):
    def announce_release(
        self,
        webhook: str,
        version_name: str,
        google_doc_url: str,
        tag_all_teams: bool,
        os_kind: OsKind,
    ) -> None:
        announce_release_on_slack(
            webhook, version_name, google_doc_url, tag_all_teams, os_kind
        )


def announce_release_on_slack(
    slack_url: str,
    version_name: str,
    google_doc_url: str,
    tag_all_teams: bool,
    os_kind: OsKind,
) -> None:
    slack = WebhookClient(
        url=slack_url, retry_handlers=[RetryHandler(max_retry_count=2)]
    )
    notify_line = (
        " ".join(
            [
                f"<!subteam^{t.slack_id}>"
                for t in RELEASE_NOTES_REVIEWERS
                if t.send_announcement
            ]
        )
        if tag_all_teams
        else "everyone"
    )
    slack.send(
        text=f"""\
Hi {notify_line}!
Here are the {os_kind} release notes for the rollout of <https://github.com/dfinity/ic/tree/{version_name}|`{version_name}`> <{google_doc_url}|Release Notes> :railway_car:
Please adjust the release notes to make sure we appropriately covered all changes made by your team since the last release, and then confirm that you reviewed the release notes by crossing out your team in the Google Doc.
""",
    )


def main() -> None:
    load_dotenv()

    announce_release_on_slack(
        os.environ["SLACK_WEBHOOK_URL"],
        "release-2024-03-06_23-01-base",
        "https://docs.google.com/document/d/1gCPmYxoq9_IccdChRzjoblAggTOdZ_IfTMukRbODO1I/edit#heading=h.7dcpz3fj7xrh",
        True,
        GUESTOS,
    )
    announce_release_on_slack(
        os.environ["SLACK_WEBHOOK_URL"],
        "release-2024-03-06_23-01-p2p",
        "https://docs.google.com/document/d/1gCPmYxoq9_IccdChRzjoblAggTOdZ_IfTMukRbODO1I/edit#heading=h.7dcpz3fj7xrh",
        False,
        GUESTOS,
    )


if __name__ == "__main__":
    main()
