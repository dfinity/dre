import http.server
import json
import logging
import socket
import threading
import traceback
import typing

import pytest
import pytest_mock
import requests

import open_messaging
from const import GUESTOS, HOSTOS
from open_messaging import OpenMessagingError

SECRET = "s3cr3t-value"
WEBHOOK = f"https://gateway.example/api/hooks/abc123/{SECRET}"
TAG = "release-2026-11-20_03-19-base"
DOC = "https://docs.google.com/document/d/1gCPmYxoq9/edit"


# --- The webhook from the environment ---------------------------------------


def test_webhook_is_read_from_the_environment(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("OPEN_MESSAGING_WEBHOOK_URL", WEBHOOK)
    assert open_messaging.webhook_from_env() == WEBHOOK


@pytest.mark.parametrize("value", [None, ""])
def test_webhook_unset_or_empty_is_none(
    monkeypatch: pytest.MonkeyPatch, value: str | None
) -> None:
    # The reconciler then skips the announcement instead of failing.
    if value is None:
        monkeypatch.delenv("OPEN_MESSAGING_WEBHOOK_URL", raising=False)
    else:
        monkeypatch.setenv("OPEN_MESSAGING_WEBHOOK_URL", value)
    assert open_messaging.webhook_from_env() is None


def test_slack_webhook_is_not_read(monkeypatch: pytest.MonkeyPatch) -> None:
    # Slack is gone: its webhook, still in the deployment's secret, is ignored.
    monkeypatch.delenv("OPEN_MESSAGING_WEBHOOK_URL", raising=False)
    monkeypatch.setenv("SLACK_WEBHOOK_URL", "https://hooks.slack.example/x")
    assert open_messaging.webhook_from_env() is None


# --- The attachment ----------------------------------------------------------


def test_payload_is_a_single_attachment() -> None:
    payload = open_messaging.payload(TAG, DOC, GUESTOS)
    # No top-level text, which Open Messaging does not render properly.
    assert list(payload) == ["attachments"]
    assert payload["attachments"] == [
        {
            "title": f"GuestOS release notes: {TAG}",
            "title_link": f"https://github.com/dfinity/ic/tree/{TAG}",
            "text": (
                "Good news! Here are the GuestOS release notes for the rollout"
                f" of `{TAG}`: <{DOC}|Release Notes>"
            ),
            "fallback": f"GuestOS release notes: {TAG}",
            "color": "good",
            "mrkdwn_in": ["text", "fallback"],
        }
    ]


def test_payload_names_the_os_kind() -> None:
    attachment = open_messaging.payload(TAG, DOC, HOSTOS)["attachments"][0]
    assert attachment["title"] == f"HostOS release notes: {TAG}"
    assert "HostOS release notes for the rollout" in attachment["text"]


def test_payload_has_no_mentions_or_emoji() -> None:
    rendered = json.dumps(open_messaging.payload(TAG, DOC, GUESTOS))
    assert "<!" not in rendered
    assert "<@" not in rendered
    assert ":railway_car:" not in rendered


# --- Posting to the gateway ----------------------------------------------------


class _Response:
    def __init__(self, status_code: int, text: str) -> None:
        self.status_code = status_code
        self.text = text


@pytest.fixture
def no_sleep(mocker: pytest_mock.MockerFixture) -> typing.Any:
    return mocker.patch("open_messaging.time.sleep")


def test_post_accepts_ok(mocker: pytest_mock.MockerFixture) -> None:
    post = mocker.patch(
        "open_messaging.requests.post", return_value=_Response(200, "ok\n")
    )
    payload = open_messaging.payload(TAG, DOC, GUESTOS)
    open_messaging.post(WEBHOOK, payload)
    post.assert_called_once_with(
        WEBHOOK, json=payload, timeout=open_messaging.TIMEOUT_SECONDS
    )


def test_post_rejects_a_200_that_is_not_ok(mocker: pytest_mock.MockerFixture) -> None:
    mocker.patch(
        "open_messaging.requests.post",
        return_value=_Response(200, "<html>welcome</html>"),
    )
    with pytest.raises(OpenMessagingError, match="answered 200: <html>welcome"):
        open_messaging.post(WEBHOOK, {})


def test_post_redacts_an_error_body_that_echoes_the_webhook(
    mocker: pytest_mock.MockerFixture,
) -> None:
    mocker.patch(
        "open_messaging.requests.post",
        return_value=_Response(404, f"no route for /api/hooks/abc123/{SECRET}"),
    )
    with pytest.raises(OpenMessagingError) as raised:
        open_messaging.post(WEBHOOK, {})
    assert (
        str(raised.value)
        == "Open Messaging webhook answered 404: no route for <redacted>"
    )


def test_post_cuts_a_long_error_body(mocker: pytest_mock.MockerFixture) -> None:
    mocker.patch(
        "open_messaging.requests.post", return_value=_Response(500, "x" * 10_000)
    )
    with pytest.raises(OpenMessagingError) as raised:
        open_messaging.post(WEBHOOK, {})
    assert len(str(raised.value)) < open_messaging.MAX_ERROR_BODY + 50


def test_post_retries_a_failure_to_connect(
    mocker: pytest_mock.MockerFixture, no_sleep: typing.Any
) -> None:
    post = mocker.patch(
        "open_messaging.requests.post",
        side_effect=[
            requests.ConnectionError(f"cannot connect to {WEBHOOK}"),
            _Response(200, "ok"),
        ],
    )
    open_messaging.post(WEBHOOK, {})
    assert post.call_count == 2


def test_post_gives_up_after_the_retries(
    mocker: pytest_mock.MockerFixture, no_sleep: typing.Any
) -> None:
    post = mocker.patch(
        "open_messaging.requests.post",
        side_effect=requests.ConnectionError(f"cannot connect to {WEBHOOK}"),
    )
    with pytest.raises(OpenMessagingError):
        open_messaging.post(WEBHOOK, {})
    assert post.call_count == 1 + open_messaging.MAX_RETRIES


def test_post_does_not_retry_a_post_that_may_have_arrived(
    mocker: pytest_mock.MockerFixture, no_sleep: typing.Any
) -> None:
    post = mocker.patch(
        "open_messaging.requests.post",
        side_effect=requests.ReadTimeout(f"read timed out from {WEBHOOK}"),
    )
    with pytest.raises(OpenMessagingError):
        open_messaging.post(WEBHOOK, {})
    assert post.call_count == 1


def test_announcer_posts_the_announcement(mocker: pytest_mock.MockerFixture) -> None:
    post = mocker.patch(
        "open_messaging.requests.post", return_value=_Response(200, "ok")
    )
    open_messaging.OpenMessagingAnnouncer().announce_release(
        webhook=WEBHOOK, version_name=TAG, google_doc_url=DOC, os_kind=GUESTOS
    )
    post.assert_called_once_with(
        WEBHOOK,
        json=open_messaging.payload(TAG, DOC, GUESTOS),
        timeout=open_messaging.TIMEOUT_SECONDS,
    )


def test_announcer_raises_a_failure(mocker: pytest_mock.MockerFixture) -> None:
    # Raised, so that the reconciler marks the announcement phase as failed.
    mocker.patch("open_messaging.requests.post", return_value=_Response(502, "down"))
    with pytest.raises(OpenMessagingError, match="answered 502: down"):
        open_messaging.OpenMessagingAnnouncer().announce_release(
            webhook=WEBHOOK, version_name=TAG, google_doc_url=DOC, os_kind=GUESTOS
        )


class _Gateway(http.server.BaseHTTPRequestHandler):
    """Answers like the Open Messaging gateway, and keeps what it was sent."""

    received: typing.ClassVar[list[tuple[str, str, typing.Any]]] = []

    def do_POST(self) -> None:
        body = self.rfile.read(int(self.headers["Content-Length"]))
        self.received.append(
            (self.path, self.headers["Content-Type"], json.loads(body))
        )
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b"ok")

    def log_message(self, format: str, *args: typing.Any) -> None:
        pass


def test_announcement_reaches_the_gateway_as_json() -> None:
    _Gateway.received = []
    server = http.server.HTTPServer(("127.0.0.1", 0), _Gateway)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        url = f"http://127.0.0.1:{server.server_port}/api/hooks/abc123/{SECRET}"
        open_messaging.announce_release(url, TAG, DOC, GUESTOS)
    finally:
        server.shutdown()
        server.server_close()
    assert _Gateway.received == [
        (
            f"/api/hooks/abc123/{SECRET}",
            "application/json",
            open_messaging.payload(TAG, DOC, GUESTOS),
        )
    ]


def test_a_real_connection_error_is_redacted_and_raised_without_its_cause(
    no_sleep: typing.Any, caplog: pytest.LogCaptureFixture
) -> None:
    # A port nobody listens on: requests names the address in its error.
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        port = s.getsockname()[1]
    url = f"http://127.0.0.1:{port}/api/hooks/abc123/{SECRET}"
    with caplog.at_level(logging.DEBUG), pytest.raises(OpenMessagingError) as raised:
        open_messaging.post(url, {})
    assert "<redacted>" in str(raised.value)
    assert raised.value.__cause__ is None
    assert raised.value.__suppress_context__
    assert SECRET not in "".join(traceback.format_exception(raised.value))
    assert SECRET not in caplog.text


# --- Redaction -----------------------------------------------------------------


def test_redact_removes_the_address_the_path_and_the_secret() -> None:
    # A gateway may echo any of the three back in an error body.
    assert (
        open_messaging.redact(WEBHOOK, f"posting to {WEBHOOK} failed")
        == "posting to <redacted> failed"
    )
    assert (
        open_messaging.redact(WEBHOOK, f"no route for /api/hooks/abc123/{SECRET}")
        == "no route for <redacted>"
    )
    assert (
        open_messaging.redact(WEBHOOK, f"unknown hook {SECRET}")
        == "unknown hook <redacted>"
    )


def test_redact_leaves_unrelated_text_alone() -> None:
    assert open_messaging.redact(WEBHOOK, "502 Bad Gateway") == "502 Bad Gateway"


def test_redact_ignores_a_trailing_slash() -> None:
    assert (
        open_messaging.redact(WEBHOOK + "/", f"unknown hook {SECRET}")
        == "unknown hook <redacted>"
    )


def test_redact_survives_an_unparsable_address() -> None:
    assert open_messaging.redact("not a url", "502 Bad Gateway") == "502 Bad Gateway"
