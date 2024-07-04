from slack_bolt.app.async_app import AsyncApp
from slack_bolt.adapter.socket_mode.async_handler import AsyncSocketModeHandler
from slackblocks import DividerBlock, SectionBlock, ActionsBlock, Button, Text, Message
import logging
import requests
import json
from datetime import datetime, timedelta

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
            action_id="silence_clicked"
        )

        actions = ActionsBlock(elements=[vote_button, silence_button])
        blocks = [divider, section, actions]

        return text, blocks
    
    def create_silence(self, proposal_id):
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

        headers = {'Content-Type': 'application/json'}
        response = requests.post(f"{self.alertmanager_url}/api/v2/silences", data=json.dumps(silence), headers=headers)
        if response.status_code != 200:
            error = (f"*Failed creating silence for proposal {proposal_id}*\n" +
                    f"Please create the silence manually <{self.alertmanager_url}" +
                    f"/#/silences/new?filter=%7B{proposal_id}%3D%22130812%22%7D|*here*>")
            
            logging.error(error)
            raise Exception(error)
        else:
            logging.info(f"Silence created for proposal with id: {proposal_id}")

    async def send_message(self, text, blocks):
        message = Message(channel=self.slack_app_channel, text=text, blocks=blocks)
        await self.slack_app.client.chat_postMessage(**message)

    async def vote_blackhole(self, ack):
        await ack()

    async def silence_blackhole(self, ack):
        await ack()
        
    async def silence_clicked(self, ack, body, client, say):
        await ack()

        proposal_id = body['actions'][0]['value']
        channel = body['channel']['id']
        message_ts = body['message']['ts']
        blocks = body['message']['blocks']
        username = body["user"]["username"]
        await ack(f"Hi <@{username}>!")
        blocks[2]['elements'][1] = Button(
                                        text=Text(f"Silenced by <@{username}> :no_bell:", emoji=True),
                                        value=proposal_id,
                                        action_id="silence_blackhole",
                                        url=f"{self.alertmanager_url}/#/silences",
                                        style="primary"
                                    )._resolve()
        try:
            self.create_silence(proposal_id)
        except Exception as e:
            await say(str(e))
        else:
            # Update the message with button clicked
            await client.chat_update(
                channel=channel,
                ts=message_ts,
                text=f"The alert with Proposal ID {proposal_id} has been silenced.",
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
        self.slack_app.action("silence_clicked")(self.silence_clicked)
        self.slack_app.action("silence_blackhole")(self.silence_blackhole)
        self.handler = AsyncSocketModeHandler(self.slack_app, slack_app_token)
