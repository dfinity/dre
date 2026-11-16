import re
import pytest
from unittest.mock import AsyncMock, MagicMock, patch
from aiohttp import web
from aiohttp.test_utils import TestServer

from alerts_receiver import AlertsReceiver
from open_messaging import OpenMessaging, OpenMessagingError

ALERTMANAGER_URL = "https://alertmanager.example.com/"
HOOK_PATH = "/api/hooks/WH-0001/not-a-real-secret"

MOCKED_TEST_ALERT = {
    "alerts": [
        {
            "labels": {
                "alertname": "ExpiresInLessThan2Hours",
                "proposal_id": "123",
                "proposal_topic": "TOPIC_NEURON_MANAGEMENT",
                "proposal_type": "ACTION_MANAGE_NEURON",
            }
        }
    ]
}


def test_silence_url_prefills_the_proposal_matcher():
    om = OpenMessaging("https://om.example.com" + HOOK_PATH, ALERTMANAGER_URL)
    assert om.silence_url("123") == (
        "https://alertmanager.example.com/#/silences/new?filter=%7Bproposal_id%3D%22123%22%7D"
    )


def test_format_alert():
    om = OpenMessaging("https://om.example.com" + HOOK_PATH, ALERTMANAGER_URL)
    attachment = om.format_alert("ExpiresInLessThan2Hours", "123", "TOPIC_NEURON_MANAGEMENT", "ACTION_MANAGE_NEURON")[
        "attachments"
    ][0]

    assert attachment["title"] == "Proposal 123: ExpiresInLessThan2Hours"
    assert attachment["title_link"] == "https://dashboard.internetcomputer.org/proposal/123"
    assert attachment["color"] == "danger"
    assert "*Topic:* TOPIC_NEURON_MANAGEMENT" in attachment["text"]
    assert "*Type:* ACTION_MANAGE_NEURON" in attachment["text"]
    assert "<https://nns.icp.net/voting/proposals/123|Vote>" in attachment["text"]
    assert f"<{om.silence_url('123')}|Silence>" in attachment["text"]
    # Open Messaging prints :emoji: codes as text, so there must be none.
    assert re.search(r":[a-z0-9_+-]+:", attachment["text"]) is None


def test_format_alert_reminders_are_amber():
    om = OpenMessaging("https://om.example.com" + HOOK_PATH, ALERTMANAGER_URL)
    payload = om.format_alert("ExpiresTomorrowBefore7pm", "123", "t", "t")
    assert payload["attachments"][0]["color"] == "warning"


async def _serve(answer_status: int, answer_text: str):
    received = []

    async def hook(request):
        received.append(await request.json())
        return web.Response(status=answer_status, text=answer_text)

    app = web.Application()
    app.router.add_post(HOOK_PATH, hook)
    return TestServer(app), received


@pytest.mark.asyncio
async def test_send_alert_posts_the_payload():
    server, received = await _serve(200, "ok")
    async with server:
        om = OpenMessaging(str(server.make_url(HOOK_PATH)), ALERTMANAGER_URL)
        await om.send_alert("ExpiresInLessThan2Hours", "123", "TOPIC_NEURON_MANAGEMENT", "ACTION_MANAGE_NEURON")

    assert received == [
        om.format_alert("ExpiresInLessThan2Hours", "123", "TOPIC_NEURON_MANAGEMENT", "ACTION_MANAGE_NEURON")
    ]


@pytest.mark.asyncio
@pytest.mark.parametrize("status,text", [(404, '{"error":"not found"}'), (200, "not ok")])
async def test_send_alert_rejects_anything_but_ok_without_leaking_the_address(status, text):
    server, _ = await _serve(status, text)
    async with server:
        om = OpenMessaging(str(server.make_url(HOOK_PATH)), ALERTMANAGER_URL)
        with pytest.raises(OpenMessagingError) as e:
            await om.send_alert("ExpiresInLessThan2Hours", "123", "t", "t")

    assert str(status) in str(e.value)
    assert "not-a-real-secret" not in str(e.value)


@pytest.mark.asyncio
@pytest.mark.parametrize(
    "echo",
    [
        lambda request: f"Cannot POST {request.path}",
        lambda request: f'{{"error": "bad hook", "url": "{request.url}"}}',
        # The secret straddles the 200-character cut, so only redacting first keeps all of it out.
        lambda request: "x" * 190 + request.path.rsplit("/", 1)[-1],
    ],
)
async def test_send_alert_redacts_an_echoed_address(echo):
    async def hook(request):
        return web.Response(status=404, text=echo(request))

    app = web.Application()
    app.router.add_post(HOOK_PATH, hook)
    async with TestServer(app) as server:
        om = OpenMessaging(str(server.make_url(HOOK_PATH)), ALERTMANAGER_URL)
        with pytest.raises(OpenMessagingError) as e:
            await om.send_alert("ExpiresInLessThan2Hours", "123", "t", "t")

    message = str(e.value)
    assert "404" in message
    assert "<redacted" in message
    for fragment in ("not-a-real-secret", "not-a-real", "real-secret", HOOK_PATH):
        assert fragment not in message


@pytest.mark.asyncio
async def test_send_alert_unreachable_does_not_leak_the_address():
    om = OpenMessaging("http://127.0.0.1:9" + HOOK_PATH, ALERTMANAGER_URL, timeout_seconds=5)
    with pytest.raises(OpenMessagingError) as e:
        await om.send_alert("ExpiresInLessThan2Hours", "123", "t", "t")
    assert "not-a-real-secret" not in str(e.value)


def test_receiver_needs_exactly_one_destination():
    with patch("alerts_receiver.GovernanceCanister"):
        with pytest.raises(ValueError):
            AlertsReceiver()
        with pytest.raises(ValueError):
            AlertsReceiver(slack_app=AsyncMock(), open_messaging=AsyncMock())


@pytest.fixture
def om_receiver():
    with patch("alerts_receiver.GovernanceCanister"):
        open_messaging = AsyncMock()
        receiver = AlertsReceiver(open_messaging=open_messaging)
    receiver.governance_canister = MagicMock()
    return receiver, open_messaging


@pytest.mark.asyncio
async def test_alert_sent_to_open_messaging(om_receiver):
    receiver, open_messaging = om_receiver
    receiver.governance_canister.has_dfinity_voted.return_value = False

    response = await receiver.server.test_client().post("/alert", json=MOCKED_TEST_ALERT)

    assert response.status_code == 200
    open_messaging.send_alert.assert_awaited_once_with(
        "ExpiresInLessThan2Hours", "123", "TOPIC_NEURON_MANAGEMENT", "ACTION_MANAGE_NEURON"
    )


@pytest.mark.asyncio
async def test_alert_not_sent_to_open_messaging_once_dfinity_voted(om_receiver):
    receiver, open_messaging = om_receiver
    receiver.governance_canister.has_dfinity_voted.return_value = True

    response = await receiver.server.test_client().post("/alert", json=MOCKED_TEST_ALERT)

    assert response.status_code == 200
    open_messaging.send_alert.assert_not_called()


@pytest.mark.asyncio
async def test_open_messaging_failure_makes_alertmanager_retry(om_receiver):
    receiver, open_messaging = om_receiver
    receiver.governance_canister.has_dfinity_voted.return_value = False
    open_messaging.send_alert.side_effect = OpenMessagingError("Open Messaging webhook answered 503: down")

    response = await receiver.server.test_client().post("/alert", json=MOCKED_TEST_ALERT)

    assert response.status_code == 502
