import os
from slack_bolt import App
from slack_bolt.adapter.socket_mode import SocketModeHandler
import requests
import json
from datetime import datetime, timedelta

# Slack Bolt app
slack_app = App(token=os.environ["SLACK_BOT_TOKEN"])

ALERTMANAGER_URL = os.environ["ALERTMANAGER_URL"]

class SlackConnect:
    """Client for managing release notes in Google Drive."""

    def create_silence(proposal_id):
        now = datetime.now()
        ends_at = now + timedelta(weeks=1)

        silence = {
            "matchers": [
                {
                    "name": "proposal_id",
                    "value": proposal_id,
                    "isRegex": False
                }
            ],
            "startsAt": now.isoformat() + "Z",
            "endsAt": ends_at.isoformat() + "Z",
            "createdBy": "slack-bot",
            "comment": "Silenced via Slack"
        }

        headers = {'Content-Type': 'application/json'}
        response = requests.post(f"{ALERTMANAGER_URL}/api/v2/silences", data=json.dumps(silence), headers=headers)
        if response.status_code != 200:
            print(f"Error creating silence: {response.content}")

    @slack_app.action("blackhole")
    def blackhole(ack, body, client):
        ack()

    @slack_app.event("message")
    def handle_message_events(body, logger):
        logger.info(body)

    @slack_app.action("silence_alert")
    def handle_silence_alert(ack, body, client):
        ack()

        proposal_id = body['actions'][0]['value']
        user = body['user']['id']
        channel = body['channel']['id']
        message_ts = body['message']['ts']
        blocks = body['message']['blocks']



        # Find the index of the Silence button block
        for block in blocks:
            if block.get('type') == 'actions':
                elements = block.get('elements', [])
                for i, element in enumerate(elements):
                    if element.get('action_id') == 'silence_alert' and element.get('value') == proposal_id:
                        # Disable the Silence button
                        elements[i] = {
                            "type": "button",
                            "text": {
                                "type": "plain_text",
                                "text": f"Silenced :no_bell:",
                                "emoji": True
                            },
                            "value": proposal_id,
                            "action_id": "silence_alert",
                            "url": f"https://alertmanager.mainnet.dfinity.network/#/silences",
                            "style": "primary"
                        }

        #create_silence(proposal_id):
        update_message = f"The alert with Proposal ID {proposal_id} has been silenced."

        client.chat_update(
            channel=channel,
            ts=message_ts,
            text=update_message,
            blocks=blocks  # Use the modified blocks
        )

    def __init__(self): 
        handler = SocketModeHandler(slack_app, os.environ["SLACK_APP_TOKEN"])
        handler.connect()
