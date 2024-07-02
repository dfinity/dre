import os
import sys
sys.path.append(os.path.join(os.path.dirname(__file__)))

from slack_app import SlackApp
from flask import Flask, request, jsonify

class AlertsReceiver:
    def __init__(self, slack_app: SlackApp):
        self.slack_app = slack_app

        self.server = Flask(__name__)
        self.server.add_url_rule('/alert', 'alert', self.alert_handler, methods=['POST'])

    def alert_handler(self):
        data = request.json
        if data:
            for alert in data.get('alerts', []):
                text, blocks = self.slack_app.format_alert(
                    proposal_id = alert['labels']['proposal_id'],
                    proposal_topic = alert['labels'].get('proposal_topic', 'Unknown'),
                    proposal_type = alert['labels'].get('proposal_type', 'Unknown')
                )
                self.slack_app.send_message(text, blocks)
            return jsonify({"status": "success"}), 200
        else:
            return jsonify({"error": "no data"}), 400

def main():
    slack_app = SlackApp(
        os.environ["ALERTMANAGER_URL"], 
        os.environ["SLACK_BOT_TOKEN"],
        os.environ["SLACK_APP_TOKEN"] 
    )
    alerts_handler = AlertsReceiver(slack_app)

    slack_app.handler.connect()
    alerts_handler.server.run(host='0.0.0.0', port=5001)

if __name__ == "__main__":
    main()
