
# Trusted Neurons Alerts

## Overview

The Trusted Neurons Alerts App is designed to receive alert notifications of `#SoonToExpireProposals` from an Alertmanager instance and push corresponding messages to a specified Slack channel.

The user can then press on `Vote` button to place his/her vote on the proposal or on `Silence` button to add a silence in Alertmanager for all the alerts on the proposal.

## Workflow

- *Receive Alert:* The app receives an HTTP POST request from Alertmanager containing alert details.
- *Parse Alert:* The alert data is parsed to extract relevant information.
- *Push to Slack:* The extracted information is formatted into a message and pushed to a specified Slack channel.

## API spec

### `POST` /alert

The incoming requests from Alertmanager contain JSON payloads structured as follows:

#### Alert Payload Example

```JSON
{
    "alerts": [
        {
            "labels": {
                "alertname": "ALERT_NAME",
                "proposal_id": "123456",
                "proposal_topic": "PROPOSAL_TOPIC",
                "proposal_type": "PROPOSAL_TYPE",
                ...
            },
            ...
        }
    ],
    ...
}
```

## Environment Variables

### ALERTMANAGER_URL

- *Description:* URL for the Alertmanager instance, used to push silences.
- *Usage:* When the user wants to silence an alert, this URL is used to communicate with Alertmanager.
- *Example:* <https://alertmanager.example.com>

### SLACK_BOT_TOKEN

- *Description:* Bot token for the Slack app, used to authenticate API requests to Slack.
- *Usage:* This token is used by the app to send messages to Slack channels. Find it in Slack App config page

### SLACK_APP_TOKEN

- *Description:* App token for the Slack app, used to configure the app and interact with the Slack API.
- *Usage:* This token is used by the app to send messages to Slack channels. Find it in Slack App config page

### SLACK_APP_CHANNEL

- *Description:* The Slack channel where alerts will be posted.
- *Usage:* Defines the target channel in the Slack workspace for the alert messages.
- *Example:* #alerts

# Troubleshooting

- *Alert Not Received:* Alerts received by the App are logged in stdout. Verify that Alertmanager is correctly configured to send alerts to the App's endpoint and that the endpoint is reachable.

- *Message Not Sent to Slack:* Check the Slack tokens `SLACK_APP_TOKEN` and `SLACK_APP_CHANNEL` and ensure the channel exists.
Check logs of the App.

- *Silencing Not Working:* Ensure the `ALERTMANAGER_URL` is correctly set and reachable.
