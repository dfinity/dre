import os
from dotenv import load_dotenv
from slack_sdk.webhook import WebhookClient
from slack_sdk.http_retry.handler import RetryHandler

teams = [
    "SRJ3R849E",  # consensus
    "SU7BZQ78E",  # crypto
    "S01A577UL56",  # execution
    "S01SVC713PS",  # messaging
    "SR6KC1DMZ",  # networking
    "S027838EY30",  # node team
    "S03BM6C0CJY",  # runtime
]


def announce_release(slack_url, version_name, google_doc_url, tag_all_teams):
    slack = WebhookClient(url=slack_url, retry_handlers=[RetryHandler(max_retry_count=2)])
    notify_line = " ".join([f"<!subteam^{t}>" for t in teams]) if tag_all_teams else "everyone"
    slack.send(
        text=f"""\
Hi {notify_line}!
Here are the release notes for the rollout of <https://github.com/dfinity/ic/tree/{version_name}|`{version_name}`> <{google_doc_url}|Release Notes> :railway_car:
Please adjust the release notes to make sure we appropriately covered all changes made by your team since the last release, and then confirm that you reviewed the release notes by crossing out your team in the Google Doc.
""",
    )


def main():
    load_dotenv()

    announce_release(
        os.environ["SLACK_WEBHOOK_URL"],
        "release-2024-03-06_23-01-base",
        "https://docs.google.com/document/d/1gCPmYxoq9_IccdChRzjoblAggTOdZ_IfTMukRbODO1I/edit#heading=h.7dcpz3fj7xrh",
        True,
    )
    announce_release(
        os.environ["SLACK_WEBHOOK_URL"],
        "release-2024-03-06_23-01-p2p",
        "https://docs.google.com/document/d/1gCPmYxoq9_IccdChRzjoblAggTOdZ_IfTMukRbODO1I/edit#heading=h.7dcpz3fj7xrh",
        False,
    )


if __name__ == "__main__":
    main()
