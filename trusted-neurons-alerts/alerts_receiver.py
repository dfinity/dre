from typing import Optional
from quart import Quart, request, jsonify
from slack_app import SlackApp
from open_messaging import OpenMessaging, OpenMessagingError
from governance import GovernanceCanister
from slackblocks import DividerBlock, SectionBlock, ActionsBlock, Button, Text, Message
import logging

class AlertsReceiver:
    """Receives alerts from Alertmanager and posts them to Slack or to Open Messaging."""

    def __init__(self, slack_app: Optional[SlackApp] = None, open_messaging: Optional[OpenMessaging] = None):
        if (slack_app is None) == (open_messaging is None):
            raise ValueError("AlertsReceiver needs exactly one of slack_app and open_messaging")
        self.slack_app = slack_app
        self.open_messaging = open_messaging
        self.governance_canister = GovernanceCanister()
        self.server = Quart(__name__)
        self.server.add_url_rule('/alert', 'alert', self.alert_handler, methods=['POST'])
    
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
            url=f"https://nns.icp.net/voting/proposals/{proposal_id}"
        )
        silence_button = Button(
            text=Text("Silence :no_bell:", emoji=True),
            value=proposal_id,
            action_id="silence_create"
        )

        actions = ActionsBlock(elements=[vote_button, silence_button])
        blocks = [divider, section, actions]

        return text, blocks

    async def alert_handler(self):
        """Parses the alert and send it to SlackApp."""

        data = await request.get_json()
        logging.debug(f"Received: {data}")
        if data:
            for alert in data.get('alerts', []):
                proposal_id = alert['labels']['proposal_id']

                if not self.governance_canister.has_dfinity_voted(int(proposal_id)):
                    alertname = alert['labels']['alertname']
                    proposal_topic = alert['labels'].get('proposal_topic', 'Unknown')
                    proposal_type = alert['labels'].get('proposal_type', 'Unknown')

                    if self.open_messaging is not None:
                        try:
                            await self.open_messaging.send_alert(alertname, proposal_id, proposal_topic, proposal_type)
                        except OpenMessagingError as e:
                            # A non-2xx answer makes Alertmanager retry the notification.
                            logging.error("Posting proposal %s to Open Messaging failed: %s", proposal_id, e)
                            return jsonify({"error": "open messaging post failed"}), 502
                        continue

                    text, blocks = self.format_alert(
                        alertname=alertname,
                        proposal_id=proposal_id,
                        proposal_topic=proposal_topic,
                        proposal_type=proposal_type
                    )

                    await self.slack_app.send_message(text, blocks)
                else:
                    logging.info("Dfinity Foundation has already voted on proposal: %s. Not sending alert.", proposal_id)
                    
            return jsonify({"status": "success"}), 200
        else:
            return jsonify({"error": "no data"}), 400
