import json
import pytest
from flask_app import flask_app

@pytest.fixture
def client():
    with flask_app.test_client() as client:
        yield client

def test_alert(client, mocker):
    # Mock data to be sent in the POST request
    mock_data = {
        "alerts": [
            {
                "labels": {
                    "proposal_id": "12345",
                    "alertname": "TestAlert",
                    "severity": "critical"
                },
                "annotations": {
                    "description": "This is a test alert"
                }
            }
        ]
    }

    # Mock the send_to_slack function
    mock_send_to_slack = mocker.patch('flask_app.send_to_slack')

    # Make the POST request to the /alert route
    response = client.post('/alert', data=json.dumps(mock_data), content_type='application/json')

    # Assert the response
    assert response.status_code == 200
    assert response.json == {"status": "success"}

    # Check that send_to_slack was called with the expected message and blocks
    expected_message = (
        "*Alert:* TestAlert\n"
        "*Severity:* critical\n"
        "*Description:* This is a test alert\n"
        "*Proposal ID:* 12345"
    )
    expected_blocks = [
        {
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": expected_message
            }
        },
        {
            "type": "actions",
            "elements": [
                {
                    "type": "button",
                    "text": {
                        "type": "plain_text",
                        "text": "Silence"
                    },
                    "value": "12345",
                    "action_id": "silence_alert"
                }
            ]
        }
    ]

    mock_send_to_slack.assert_called_once_with(expected_message, expected_blocks)
