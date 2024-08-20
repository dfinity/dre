from quart import Quart, request, jsonify
from slack_app import SlackApp
from governance import GovernanceCanister
from slackblocks import DividerBlock, SectionBlock, ActionsBlock, Button, Text, Message
import logging

class AlertsReceiver:
    """Receives alerts from Alertmanager."""
    
    def __init__(self, slack_app: SlackApp):
        self.slack_app = slack_app
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

    async def alert_handler(self):
        """Parses the alert and send it to SlackApp."""

        data = await request.get_json()
        logging.debug(f"Received: {data}")
        if data:
            for alert in data.get('alerts', []):
                proposal_id = alert['labels']['proposal_id']

                if not self.governance_canister.has_dfinity_voted(int(proposal_id)):
                    text, blocks = self.format_alert(
                        alertname=alert['labels']['alertname'],
                        proposal_id=proposal_id,
                        proposal_topic=alert['labels'].get('proposal_topic', 'Unknown'),
                        proposal_type=alert['labels'].get('proposal_type', 'Unknown')
                    )

                    await self.slack_app.send_message(text, blocks)
                else:
                    logging.info("Dfinity Foundation has already voted on proposal: %s. Not sending alert.", proposal_id)
                    
            return jsonify({"status": "success"}), 200
        else:
            return jsonify({"error": "no data"}), 400
