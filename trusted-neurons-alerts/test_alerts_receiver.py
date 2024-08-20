import pytest
from unittest.mock import AsyncMock, patch
from alerts_receiver import AlertsReceiver  # Replace with the correct import path
from quart.testing import QuartClient
from unittest.mock import MagicMock

MOCKED_TEST_ALERT = {
        "alerts": [
            {
                "labels": {
                    "alertname": "TestAlert",
                    "proposal_id": "123",
                    "proposal_topic": "TestTopic",
                    "proposal_type": "TestType"
                }
            }
        ]
    }

@pytest.fixture
@patch('slack_app.SlackApp', autospec=True)
def slack_app_mock(MockSlackApp):
    slack_app = AsyncMock()
    MockSlackApp.return_value = slack_app
    return slack_app

@pytest.fixture
@patch('governance.GovernanceCanister', autospec=True)
def governance_canister_mock(MockGovernanceCanister):
    governance_canister = MagicMock()
    MockGovernanceCanister.return_value = governance_canister
    return governance_canister

@pytest.fixture
def alerts_receiver(slack_app_mock, governance_canister_mock):
    receiver = AlertsReceiver(slack_app=slack_app_mock)
    receiver.governance_canister = governance_canister_mock
    return receiver

@pytest.fixture
def quart_client(alerts_receiver) -> QuartClient:
    return alerts_receiver.server.test_client()

@pytest.mark.asyncio
async def test_alert_handler_message_not_sent(quart_client, slack_app_mock, governance_canister_mock):
    governance_canister_mock.has_dfinity_voted.return_value = True
    response = await quart_client.post('/alert', json=MOCKED_TEST_ALERT)

    assert response.status_code == 200
    slack_app_mock.send_message.assert_not_called()

@pytest.mark.asyncio
async def test_alert_handler_message_sent(quart_client, slack_app_mock, governance_canister_mock):
    governance_canister_mock.has_dfinity_voted.return_value = False
    response = await quart_client.post('/alert', json=MOCKED_TEST_ALERT)

    assert response.status_code == 200
    slack_app_mock.send_message.assert_called()
