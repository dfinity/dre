from slack_bolt import App
from slack_bolt.adapter.socket_mode import SocketModeHandler
from slackblocks import DividerBlock, SectionBlock, ActionsBlock, Button, Text, Message
import requests
import json
from datetime import datetime, timedelta

class SlackApp:
    def format_alert(self, proposal_id: str, proposal_topic: str, proposal_type: str):
        text = (
            f"<https://dashboard.internetcomputer.org/proposal/{proposal_id}|*Proposal:* {proposal_id}>\n"
            f"*Proposal Topic:* {proposal_topic}\n"
            f"*Proposal Type:* {proposal_type}"
        )
        divider = DividerBlock()
        section = SectionBlock(Text(text))
        vote_button = Button(
            text=Text("Vote :judge:", emoji=True),
            value=proposal_id,
            action_id="vote_blackhole",
            url=f"https://nns.ic0.app/proposal/?u=qoctq-giaaa-aaaaa-aaaea-cai&proposal={proposal_id}"
        )
        silence_button = Button(
            text=Text("Silence :no_bell:", emoji=True),
            value=proposal_id,
            action_id="silence_pressed"
        )

        actions = ActionsBlock(elements=[vote_button, silence_button])
        blocks = [divider, section, actions]

        return text, blocks
    
    def create_silence(self, proposal_id):
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
        response = requests.post(f"https://alertmanager.mainnet.dfinity.network/api/v2/silences", data=json.dumps(silence), headers=headers)
        if response.status_code != 200:
            print(f"Error creating silence: {response.content}")

            
    def vote_blackhole(self, ack):
        ack()

    def handle_message_events(self, body, logger):
        logger.info(body)

    def silence_blackhole(self, ack):
        ack()
        
    def silence_pressed(self, ack, body, client):
        ack()

        proposal_id = body['actions'][0]['value']
        channel = body['channel']['id']
        message_ts = body['message']['ts']
        blocks = body['message']['blocks']
        
        for block in blocks:
            if block.get('type') == 'actions':
                elements = block.get('elements', [])
                for i, element in enumerate(elements):
                    if element.get('action_id') == 'silence_pressed' and element.get('value') == proposal_id:
                        elements[i] = {
                                        "type": "button",
                                        "text": {
                                            "type": "plain_text",
                                            "text": f"Silenced :no_bell:",
                                            "emoji": True
                                        },
                                        "action_id": "silence_blackhole",
                                        "url": f"https://alertmanager.mainnet.dfinity.network/#/silences",
                                        "style": "primary"
                                    }

        self.create_silence(proposal_id)
        update_message = f"The alert with Proposal ID {proposal_id} has been silenced."

        client.chat_update(
            channel=channel,
            ts=message_ts,
            text=update_message,
            blocks=blocks  # Use the modified blocks
        )
    
    def send_message(self, text, blocks):
            message = Message(channel="#test-app", text=text, blocks=blocks)
            self.slack_app.client.chat_postMessage(**message)

    def __init__(self, alertmanager_url: str, slack_bot_token: str, slack_app_token: str): 
        self.alertmanager_url = alertmanager_url
        self.slack_app = App(token=slack_bot_token)
                  
        self.slack_app.action("vote_blackhole")(self.vote_blackhole)
        self.slack_app.event("message")(self.handle_message_events)
        self.slack_app.action("silence_pressed")(self.silence_pressed)
        self.slack_app.action("silence_blackhole")(self.silence_blackhole)

        self.handler = SocketModeHandler(self.slack_app, slack_app_token)
