import os
import sys
sys.path.append(os.path.join(os.path.dirname(__file__)))

from quart import Quart, request, jsonify
import asyncio, logging
from slack_app import SlackApp

FORMAT = "[%(asctime)s] %(levelname)-8s %(message)s"
logging.basicConfig(format=FORMAT, level=logging.INFO)

class AlertsReceiver:
    def __init__(self, slack_app: SlackApp):
        self.slack_app = slack_app
        self.server = Quart(__name__)
        self.server.add_url_rule('/alert', 'alert', self.alert_handler, methods=['POST'])

    async def alert_handler(self):
        data = await request.get_json()
        if data:
            for alert in data.get('alerts', []):
                text, blocks = self.slack_app.format_alert(
                    proposal_id=alert['labels']['proposal_id'],
                    proposal_topic=alert['labels'].get('proposal_topic', 'Unknown'),
                    proposal_type=alert['labels'].get('proposal_type', 'Unknown')
                )
                await self.slack_app.send_message(text, blocks)
            return jsonify({"status": "success"}), 200
        else:
            return jsonify({"error": "no data"}), 400

async def main():
    slack_app = SlackApp(
        os.environ["ALERTMANAGER_URL"], 
        os.environ["SLACK_BOT_TOKEN"],
        os.environ["SLACK_APP_TOKEN"] 
    )
    await slack_app.handler.connect_async()
    alerts_handler = AlertsReceiver(slack_app)
    await alerts_handler.server.run_task(host='0.0.0.0', port=5001)

if __name__ == "__main__":
    asyncio.run(main())
