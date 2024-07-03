import os
import sys
sys.path.append(os.path.join(os.path.dirname(__file__)))
from slack_app import SlackApp
from alerts_receiver import AlertsReceiver
import asyncio, logging

async def main():
    FORMAT = "[%(asctime)s] %(levelname)-8s %(message)s"
    logging.basicConfig(format=FORMAT, level=logging.INFO)

    try:
        alertmanager_url = os.environ["ALERTMANAGER_URL"]
        slack_bot_token = os.environ["SLACK_BOT_TOKEN"]
        slack_app_token = os.environ["SLACK_APP_TOKEN"]
        slack_app_channel = os.environ["SLACK_APP_CHANNEL"]
    except KeyError as e:
        logging.error(f"Missing required environment variable: {e.args[0]}")
        return
    
    slack_app = SlackApp(alertmanager_url, slack_bot_token, slack_app_token, slack_app_channel)

    await slack_app.handler.connect_async()
    await AlertsReceiver(slack_app).server.run_task(host='0.0.0.0', port=5001)

if __name__ == "__main__":
    asyncio.run(main())
