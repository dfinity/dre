from quart import Quart, request, jsonify
from slack_app import SlackApp
import logging

class AlertsReceiver:
    """Receives alerts from Alertmanager."""
    
    def __init__(self, slack_app: SlackApp):
        self.slack_app = slack_app
        self.server = Quart(__name__)
        self.server.add_url_rule('/alert', 'alert', self.alert_handler, methods=['POST'])

    async def alert_handler(self):
        """Parses the alert and send it to SlackApp."""

        data = await request.get_json()
        logging.debug(f"Received: {data}")
        if data:
            for alert in data.get('alerts', []):
                text, blocks = self.slack_app.format_alert(
                    alertname=alert['labels']['alertname'],
                    proposal_id=alert['labels']['proposal_id'],
                    proposal_topic=alert['labels'].get('proposal_topic', 'Unknown'),
                    proposal_type=alert['labels'].get('proposal_type', 'Unknown')
                )
                await self.slack_app.send_message(text, blocks)
            return jsonify({"status": "success"}), 200
        else:
            return jsonify({"error": "no data"}), 400
