from slack_bolt.app.async_app import AsyncApp
from slack_bolt.adapter.socket_mode.async_handler import AsyncSocketModeHandler
from slackblocks import DividerBlock, SectionBlock, ActionsBlock, Button, Text, Message
import logging
import requests
import json
from datetime import datetime, timedelta

class SilenceException(Exception):
    def __init__(self, proposal_id: str, alertmanager_url: str, e: Exception):
        super().__init__()
        self.proposal_id = proposal_id
        self.alertmanager_url = alertmanager_url
        self.e = e

    def __str__(self) -> str:
        return self.e.__str__()
    
    def to_message(self) -> str:
        return self.e.__str__()
    
class SilenceRemovalException(SilenceException):
    def to_message(self) -> str:
        return f"Silence removal failed for proposal {self.proposal_id}. Alertmanager URL: {self.alertmanager_url}"
    
class SilenceCreationException(SilenceException):
    def to_message(self) -> str:
        return f"Silence creation failed for proposal {self.proposal_id}. Alertmanager URL: {self.alertmanager_url}"


class SlackApp:

    def format_alert(self, alertname: str, proposal_id: str, proposal_topic: str, proposal_type: str):
        """Format the alert in blocks of elements."""

        text = (
            f"<https://dashboard.internetcomputer.org/proposal/{proposal_id}|*Proposal:* {proposal_id}> *{alertname}*\n"
            f"*Topic:* {proposal_topic}\n"
            f"*Type:* {proposal_type}"
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
            action_id="silence_create"
        )

        actions = ActionsBlock(elements=[vote_button, silence_button])
        blocks = [divider, section, actions]

        return text, blocks
    
    def silence_create(self, proposal_id) -> str:
        """Handles the creation of the silence in alertmanager for the proposal_id."""
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

        response = requests.post(f"{self.alertmanager_url}/api/v2/silences", 
                                    data=json.dumps(silence), 
                                    headers={'Content-Type': 'application/json'})
        response.raise_for_status()
        silence_id = response.json().get("silenceID")
        logging.info(f"Silence with ID: {silence_id} created for proposal: {proposal_id}")
        return silence_id


    def silence_remove(self, silence_id):
        """Handles the removal of the silence in alertmanager for the silence_id."""
        
        delete_response = requests.delete(f"{self.alertmanager_url}/api/v2/silence/{silence_id}")
        delete_response.raise_for_status()
        logging.info(f"Silence removed for {silence_id}")

    async def send_message(self, text, blocks):
        message = Message(channel=self.slack_app_channel, text=text, blocks=blocks)
        await self.slack_app.client.chat_postMessage(**message)

    async def vote_blackhole(self, ack):
        await ack()
        
    async def silence_clicked(self, ack, body, client, say):
        await ack()
        channel = body['channel']['id']
        message_ts = body['message']['ts']
        blocks = body['message']['blocks']
        silence_button = blocks[2]['elements'][1]
        proposal_id = blocks[2]['elements'][0]['value']

        try:
            if silence_button['action_id'] == "silence_create":
                try:
                    silence_id = self.silence_create(proposal_id)
                except Exception as e:
                    raise SilenceCreationException(proposal_id, self.alertmanager_url, e)
                    
                username = body["user"]["username"]
                blocks[2]['elements'][1] = Button(
                    text=Text(f"Silenced by <@{username}> :no_bell:", emoji=True),
                    value=silence_id,
                    action_id="silence_remove",
                    style="primary"
                )._resolve()
                
            elif silence_button['action_id'] == "silence_remove":
                silence_id = body['actions'][0]['value']
                try:
                    self.silence_remove(silence_id)
                except Exception as e:
                    raise SilenceRemovalException(proposal_id, self.alertmanager_url, e)
                
                blocks[2]['elements'][1] = Button(
                    text=Text("Silence :no_bell:", emoji=True),
                    value=proposal_id,
                    action_id="silence_create"
                )._resolve()
            else:            
                logging.error(f"Action: {silence_button['action_id']} unhandled")
        except SilenceException as e:
            print(e)
            logging.error(f"Error in silence_clicked: {e}")
            return await say(e.to_message())

        await client.chat_update(
            channel=channel,
            ts=message_ts,
            text="fallback",
            blocks=blocks 
        )


    def __init__(self, 
                 alertmanager_url: str, 
                 slack_bot_token: str, 
                 slack_app_token: str, 
                 slack_app_channel: str
                 ): 
        
        self.slack_app_channel = slack_app_channel
        self.alertmanager_url = alertmanager_url
        self.slack_app = AsyncApp(token=slack_bot_token)

        # Setup handlers
        self.slack_app.action("vote_blackhole")(self.vote_blackhole)
        self.slack_app.action("silence_create")(self.silence_clicked)
        self.slack_app.action("silence_remove")(self.silence_clicked)
        self.handler = AsyncSocketModeHandler(self.slack_app, slack_app_token)
