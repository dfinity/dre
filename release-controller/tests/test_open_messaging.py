import http.server
import json
import logging
import socket
import threading
import time
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
def no_sleep(monkeypatch: pytest.MonkeyPatch) -> None:
    # Not by patching time.sleep, which is the one every thread shares.
    monkeypatch.setattr(open_messaging, "RETRY_DELAY_SECONDS", 0.0)


def _logged(caplog: pytest.LogCaptureFixture) -> str:
    """
    What the release controller would log.

    urllib3 names the request path in its DEBUG lines, but the release
    controller never shows those: util.conventional_logging keeps urllib3 at
    WARNING even with --verbose.
    """
    return "\n".join(
        r.getMessage()
        for r in caplog.records
        if not (r.name.startswith("urllib3") and r.levelno < logging.WARNING)
    )


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


def _closed_port() -> int:
    """A port nobody listens on."""
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return int(s.getsockname()[1])


def _refused() -> requests.RequestException:
    """The error requests really raises for a connection that was refused."""
    try:
        requests.post(f"http://127.0.0.1:{_closed_port()}/", json={}, timeout=5)
    except requests.RequestException as e:
        return e
    raise AssertionError("a closed port took the post")


def test_post_retries_a_connection_that_was_refused(
    mocker: pytest_mock.MockerFixture, no_sleep: None
) -> None:
    refused = _refused()
    assert open_messaging.never_reached_the_gateway(refused)
    post = mocker.patch(
        "open_messaging.requests.post", side_effect=[refused, _Response(200, "ok")]
    )
    open_messaging.post(WEBHOOK, {})
    assert post.call_count == 2


def test_post_gives_up_after_the_retries(
    mocker: pytest_mock.MockerFixture, no_sleep: None
) -> None:
    post = mocker.patch("open_messaging.requests.post", side_effect=_refused())
    with pytest.raises(OpenMessagingError):
        open_messaging.post(WEBHOOK, {})
    assert post.call_count == 1 + open_messaging.MAX_RETRIES


def test_post_does_not_retry_a_read_timeout(
    mocker: pytest_mock.MockerFixture, no_sleep: None
) -> None:
    post = mocker.patch(
        "open_messaging.requests.post",
        side_effect=requests.ReadTimeout(f"read timed out from {WEBHOOK}"),
    )
    with pytest.raises(OpenMessagingError):
        open_messaging.post(WEBHOOK, {})
    assert post.call_count == 1


class _MisbehavingGateway:
    """Takes each post, then misbehaves; counts the posts it took."""

    def __init__(self, misbehave: typing.Callable[[socket.socket], None]) -> None:
        self.posts = 0
        self._misbehave = misbehave
        self._listener = socket.create_server(("127.0.0.1", 0))
        port = self._listener.getsockname()[1]
        self.url = f"http://127.0.0.1:{port}/api/hooks/abc123/{SECRET}"
        threading.Thread(target=self._accept, daemon=True).start()

    def _accept(self) -> None:
        while True:
            try:
                conn, _ = self._listener.accept()
            except OSError:
                return
            # One thread per post, so that a post sent again is counted at
            # once even while the first one is still being stalled.
            threading.Thread(target=self._take, args=(conn,), daemon=True).start()

    def _take(self, conn: socket.socket) -> None:
        with conn:
            request = b""
            while b"\r\n\r\n" not in request:
                chunk = conn.recv(65536)
                if not chunk:
                    return
                request += chunk
            self.posts += 1
            self._misbehave(conn)

    def close(self) -> None:
        self._listener.close()


def _stall_the_body(conn: socket.socket) -> None:
    conn.sendall(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n")
    time.sleep(2)


def _drop_the_connection(conn: socket.socket) -> None:
    pass


@pytest.mark.parametrize("misbehave", [_stall_the_body, _drop_the_connection])
def test_post_is_not_sent_again_once_the_gateway_took_it(
    monkeypatch: pytest.MonkeyPatch,
    no_sleep: None,
    caplog: pytest.LogCaptureFixture,
    misbehave: typing.Callable[[socket.socket], None],
) -> None:
    # requests raises ConnectionError for both, as it does for a connection
    # that was refused, but the gateway has the announcement already.
    monkeypatch.setattr(open_messaging, "TIMEOUT_SECONDS", 0.5)
    gateway = _MisbehavingGateway(misbehave)
    try:
        with (
            caplog.at_level(logging.DEBUG),
            pytest.raises(OpenMessagingError) as raised,
        ):
            open_messaging.post(gateway.url, {})
    finally:
        gateway.close()
    assert gateway.posts == 1
    assert SECRET not in "".join(traceback.format_exception(raised.value))
    assert SECRET not in _logged(caplog)


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
    no_sleep: None, caplog: pytest.LogCaptureFixture
) -> None:
    # requests names the address in its error, on each retry too.
    url = f"http://127.0.0.1:{_closed_port()}/api/hooks/abc123/{SECRET}"
    with caplog.at_level(logging.DEBUG), pytest.raises(OpenMessagingError) as raised:
        open_messaging.post(url, {})
    assert "<redacted>" in str(raised.value)
    assert raised.value.__cause__ is None
    assert raised.value.__suppress_context__
    assert SECRET not in "".join(traceback.format_exception(raised.value))
    assert SECRET not in _logged(caplog)


def test_a_malformed_address_is_redacted_and_raised_without_its_cause(
    caplog: pytest.LogCaptureFixture,
) -> None:
    # requests rejects it with an error that quotes it, and urllib cannot
    # parse it to find the secret to strip.
    url = f"https://[gateway.example/api/hooks/abc123/{SECRET}"
    with caplog.at_level(logging.DEBUG), pytest.raises(OpenMessagingError) as raised:
        open_messaging.post(url, {})
    assert "malformed" in str(raised.value)
    assert raised.value.__cause__ is None
    assert raised.value.__suppress_context__
    assert SECRET not in "".join(traceback.format_exception(raised.value))
    assert SECRET not in _logged(caplog)


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


def test_redact_survives_an_address_without_a_scheme() -> None:
    assert open_messaging.redact("not a url", "502 Bad Gateway") == "502 Bad Gateway"


def test_redact_keeps_nothing_for_an_address_it_cannot_parse() -> None:
    # An unmatched "[" makes urlsplit() raise, which must not escape.
    url = f"https://[gateway.example/api/hooks/abc123/{SECRET}"
    redacted = open_messaging.redact(url, f"Invalid URL {url!r}: no host; {SECRET}")
    assert redacted == "<redacted: the webhook address is malformed>"
