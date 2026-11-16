
# Trusted Neurons Alerts

## Overview

The Trusted Neurons Alerts App is designed to receive alert notifications of `#SoonToExpireProposals` from an Alertmanager instance and push corresponding messages to a specified Slack channel.

The user can then press on `Vote` button to place his/her vote on the proposal or on `Silence` button to add a silence in Alertmanager for all the alerts on the proposal.

The app can post to an Open Messaging channel instead of Slack: set `OPEN_MESSAGING_WEBHOOK_URL` (see below). Open Messaging takes Slack's incoming-webhook format but has no interactive buttons, so there each message carries `Vote` and `Silence` as links. `Silence` opens Alertmanager's new-silence form, pre-filled with the proposal's matcher.

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

### OPEN_MESSAGING_WEBHOOK_URL

- *Description:* Address of an Open Messaging incoming webhook, `https://<gateway>/api/hooks/<id>/<secret>`. It is a secret.
- *Usage:* When set, alerts are posted there and the Slack app is not started, so the `SLACK_*` variables are not needed. Create the webhook in the channel's settings, under Integrations.

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
