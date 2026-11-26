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

import slack_announce
from const import GUESTOS, HOSTOS, OsKind
from slack_announce import OpenMessagingError

SECRET = "s3cr3t-value"
WEBHOOK = f"https://gateway.example/api/hooks/abc123/{SECRET}"
SLACK = "https://hooks.slack.example/services/T000/B000/XXXX"
TAG = "release-2026-11-20_03-19-base"
DOC = "https://docs.google.com/document/d/1gCPmYxoq9/edit"


class Call(typing.NamedTuple):
    destination: str
    webhook: str
    version_name: str
    google_doc_url: str
    os_kind: OsKind


class RecordingAnnouncer(slack_announce.SlackAnnouncerProtocol):
    """Records each post, and fails the destinations it is told to fail."""

    def __init__(
        self,
        slack_error: Exception | None = None,
        open_messaging_error: Exception | None = None,
    ) -> None:
        self.calls: list[Call] = []
        self.slack_error = slack_error
        self.open_messaging_error = open_messaging_error

    def announce_release(
        self, webhook: str, version_name: str, google_doc_url: str, os_kind: OsKind
    ) -> None:
        self.calls.append(Call("slack", webhook, version_name, google_doc_url, os_kind))
        if self.slack_error:
            raise self.slack_error

    def announce_release_on_open_messaging(
        self, webhook: str, version_name: str, google_doc_url: str, os_kind: OsKind
    ) -> None:
        self.calls.append(
            Call("open messaging", webhook, version_name, google_doc_url, os_kind)
        )
        if self.open_messaging_error:
            raise self.open_messaging_error


def _announce(
    announcer: RecordingAnnouncer,
    slack_webhook: str | None,
    open_messaging_webhook: str | None,
) -> None:
    slack_announce.announce_release(
        announcer,
        slack_webhook=slack_webhook,
        open_messaging_webhook=open_messaging_webhook,
        version_name=TAG,
        google_doc_url=DOC,
        os_kind=GUESTOS,
    )


def _destinations(announcer: RecordingAnnouncer) -> list[str]:
    return [c.destination for c in announcer.calls]


# --- The webhook from the environment ---------------------------------------


def test_open_messaging_webhook_is_read_from_the_environment(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("OPEN_MESSAGING_WEBHOOK_URL", WEBHOOK)
    assert slack_announce.open_messaging_webhook_from_env() == WEBHOOK


@pytest.mark.parametrize("value", [None, ""])
def test_open_messaging_webhook_unset_or_empty_is_none(
    monkeypatch: pytest.MonkeyPatch, value: str | None
) -> None:
    if value is None:
        monkeypatch.delenv("OPEN_MESSAGING_WEBHOOK_URL", raising=False)
    else:
        monkeypatch.setenv("OPEN_MESSAGING_WEBHOOK_URL", value)
    assert slack_announce.open_messaging_webhook_from_env() is None


# --- Routing between Slack and Open Messaging -------------------------------


def test_without_open_messaging_only_slack_is_announced_as_before() -> None:
    announcer = RecordingAnnouncer()
    _announce(announcer, slack_webhook=SLACK, open_messaging_webhook=None)
    assert announcer.calls == [Call("slack", SLACK, TAG, DOC, GUESTOS)]


def test_with_open_messaging_both_are_announced() -> None:
    announcer = RecordingAnnouncer()
    _announce(announcer, slack_webhook=SLACK, open_messaging_webhook=WEBHOOK)
    assert announcer.calls == [
        Call("slack", SLACK, TAG, DOC, GUESTOS),
        Call("open messaging", WEBHOOK, TAG, DOC, GUESTOS),
    ]


def test_open_messaging_alone_is_announced_once_slack_is_gone() -> None:
    announcer = RecordingAnnouncer()
    _announce(announcer, slack_webhook=None, open_messaging_webhook=WEBHOOK)
    assert _destinations(announcer) == ["open messaging"]


def test_neither_destination_announces_nothing() -> None:
    announcer = RecordingAnnouncer()
    _announce(announcer, slack_webhook=None, open_messaging_webhook=None)
    assert announcer.calls == []


def test_a_slack_failure_without_open_messaging_is_raised_unchanged() -> None:
    error = RuntimeError("slack is down")
    announcer = RecordingAnnouncer(slack_error=error)
    with pytest.raises(RuntimeError) as raised:
        _announce(announcer, slack_webhook=SLACK, open_messaging_webhook=None)
    assert raised.value is error


def test_a_slack_failure_does_not_stop_open_messaging() -> None:
    error = RuntimeError("slack is down")
    announcer = RecordingAnnouncer(slack_error=error)
    with pytest.raises(RuntimeError) as raised:
        _announce(announcer, slack_webhook=SLACK, open_messaging_webhook=WEBHOOK)
    # Open Messaging was still posted to; the phase is still marked failed.
    assert _destinations(announcer) == ["slack", "open messaging"]
    assert raised.value is error


def test_an_open_messaging_failure_does_not_stop_slack() -> None:
    announcer = RecordingAnnouncer(
        open_messaging_error=OpenMessagingError("gateway answered 502")
    )
    with pytest.raises(OpenMessagingError, match="gateway answered 502"):
        _announce(announcer, slack_webhook=SLACK, open_messaging_webhook=WEBHOOK)
    assert _destinations(announcer) == ["slack", "open messaging"]


def test_both_failing_raises_both() -> None:
    announcer = RecordingAnnouncer(
        slack_error=RuntimeError("slack is down"),
        open_messaging_error=OpenMessagingError("gateway answered 502"),
    )
    with pytest.raises(ExceptionGroup) as raised:
        _announce(announcer, slack_webhook=SLACK, open_messaging_webhook=WEBHOOK)
    assert [str(e) for e in raised.value.exceptions] == [
        "slack is down",
        "gateway answered 502",
    ]


def test_an_open_messaging_failure_is_redacted_before_it_is_logged_or_raised(
    caplog: pytest.LogCaptureFixture,
) -> None:
    # An error from a client, not built by this module, that holds the
    # address.
    announcer = RecordingAnnouncer(
        open_messaging_error=RuntimeError(f"cannot post to {WEBHOOK}")
    )
    with caplog.at_level(logging.DEBUG):
        with pytest.raises(OpenMessagingError) as raised:
            _announce(announcer, slack_webhook=None, open_messaging_webhook=WEBHOOK)
    assert str(raised.value) == "cannot post to <redacted>"
    assert SECRET not in "".join(traceback.format_exception(raised.value))
    assert SECRET not in caplog.text


# --- The attachment ----------------------------------------------------------


def test_open_messaging_payload_is_a_single_attachment() -> None:
    payload = slack_announce.open_messaging_payload(TAG, DOC, GUESTOS)
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


def test_open_messaging_payload_names_the_os_kind() -> None:
    attachment = slack_announce.open_messaging_payload(TAG, DOC, HOSTOS)["attachments"][
        0
    ]
    assert attachment["title"] == f"HostOS release notes: {TAG}"
    assert "HostOS release notes for the rollout" in attachment["text"]


def test_open_messaging_payload_drops_mentions_and_emoji() -> None:
    rendered = json.dumps(slack_announce.open_messaging_payload(TAG, DOC, GUESTOS))
    assert "<!subteam^" not in rendered
    assert "<!" not in rendered
    assert ":railway_car:" not in rendered


# --- Posting to the gateway ----------------------------------------------------


class _Response:
    def __init__(self, status_code: int, text: str) -> None:
        self.status_code = status_code
        self.text = text


@pytest.fixture
def no_sleep(mocker: pytest_mock.MockerFixture) -> typing.Any:
    return mocker.patch("slack_announce.time.sleep")


def test_post_accepts_ok(mocker: pytest_mock.MockerFixture) -> None:
    post = mocker.patch(
        "slack_announce.requests.post", return_value=_Response(200, "ok\n")
    )
    payload = slack_announce.open_messaging_payload(TAG, DOC, GUESTOS)
    slack_announce.post_to_open_messaging(WEBHOOK, payload)
    post.assert_called_once_with(
        WEBHOOK, json=payload, timeout=slack_announce.OPEN_MESSAGING_TIMEOUT_SECONDS
    )


def test_post_rejects_a_200_that_is_not_ok(mocker: pytest_mock.MockerFixture) -> None:
    mocker.patch(
        "slack_announce.requests.post",
        return_value=_Response(200, "<html>welcome</html>"),
    )
    with pytest.raises(OpenMessagingError, match="answered 200: <html>welcome"):
        slack_announce.post_to_open_messaging(WEBHOOK, {})


def test_post_redacts_an_error_body_that_echoes_the_webhook(
    mocker: pytest_mock.MockerFixture,
) -> None:
    mocker.patch(
        "slack_announce.requests.post",
        return_value=_Response(404, f"no route for /api/hooks/abc123/{SECRET}"),
    )
    with pytest.raises(OpenMessagingError) as raised:
        slack_announce.post_to_open_messaging(WEBHOOK, {})
    assert (
        str(raised.value)
        == "Open Messaging webhook answered 404: no route for <redacted>"
    )


def test_post_cuts_a_long_error_body(mocker: pytest_mock.MockerFixture) -> None:
    mocker.patch(
        "slack_announce.requests.post", return_value=_Response(500, "x" * 10_000)
    )
    with pytest.raises(OpenMessagingError) as raised:
        slack_announce.post_to_open_messaging(WEBHOOK, {})
    assert len(str(raised.value)) < slack_announce.MAX_ERROR_BODY + 50


def test_post_retries_a_failure_to_connect(
    mocker: pytest_mock.MockerFixture, no_sleep: typing.Any
) -> None:
    post = mocker.patch(
        "slack_announce.requests.post",
        side_effect=[
            requests.ConnectionError(f"cannot connect to {WEBHOOK}"),
            _Response(200, "ok"),
        ],
    )
    slack_announce.post_to_open_messaging(WEBHOOK, {})
    assert post.call_count == 2


def test_post_gives_up_after_the_retries(
    mocker: pytest_mock.MockerFixture, no_sleep: typing.Any
) -> None:
    post = mocker.patch(
        "slack_announce.requests.post",
        side_effect=requests.ConnectionError(f"cannot connect to {WEBHOOK}"),
    )
    with pytest.raises(OpenMessagingError):
        slack_announce.post_to_open_messaging(WEBHOOK, {})
    assert post.call_count == 1 + slack_announce.OPEN_MESSAGING_MAX_RETRIES


def test_post_does_not_retry_a_post_that_may_have_arrived(
    mocker: pytest_mock.MockerFixture, no_sleep: typing.Any
) -> None:
    post = mocker.patch(
        "slack_announce.requests.post",
        side_effect=requests.ReadTimeout(f"read timed out from {WEBHOOK}"),
    )
    with pytest.raises(OpenMessagingError):
        slack_announce.post_to_open_messaging(WEBHOOK, {})
    assert post.call_count == 1


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
        slack_announce.announce_release_on_open_messaging(url, TAG, DOC, GUESTOS)
    finally:
        server.shutdown()
        server.server_close()
    assert _Gateway.received == [
        (
            f"/api/hooks/abc123/{SECRET}",
            "application/json",
            slack_announce.open_messaging_payload(TAG, DOC, GUESTOS),
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
    with caplog.at_level(logging.DEBUG):
        with pytest.raises(OpenMessagingError) as raised:
            slack_announce.post_to_open_messaging(url, {})
    assert "<redacted>" in str(raised.value)
    assert raised.value.__cause__ is None
    assert raised.value.__suppress_context__
    assert SECRET not in "".join(traceback.format_exception(raised.value))
    assert SECRET not in caplog.text


# --- Redaction -----------------------------------------------------------------


def test_redact_removes_the_address_the_path_and_the_secret() -> None:
    # A gateway may echo any of the three back in an error body.
    assert (
        slack_announce.redact(WEBHOOK, f"posting to {WEBHOOK} failed")
        == "posting to <redacted> failed"
    )
    assert (
        slack_announce.redact(WEBHOOK, f"no route for /api/hooks/abc123/{SECRET}")
        == "no route for <redacted>"
    )
    assert (
        slack_announce.redact(WEBHOOK, f"unknown hook {SECRET}")
        == "unknown hook <redacted>"
    )


def test_redact_leaves_unrelated_text_alone() -> None:
    assert slack_announce.redact(WEBHOOK, "502 Bad Gateway") == "502 Bad Gateway"


def test_redact_ignores_a_trailing_slash() -> None:
    assert (
        slack_announce.redact(WEBHOOK + "/", f"unknown hook {SECRET}")
        == "unknown hook <redacted>"
    )


def test_redact_survives_an_unparsable_address() -> None:
    assert slack_announce.redact("not a url", "502 Bad Gateway") == "502 Bad Gateway"
