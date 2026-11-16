import os
import sys
from typing import Optional
sys.path.append(os.path.join(os.path.dirname(__file__)))
from slack_app import SlackApp
from open_messaging import OpenMessaging
from alerts_receiver import AlertsReceiver
import asyncio, logging

def set_logging_level(level: Optional[str] = None):
    FORMAT = "[%(asctime)s] %(levelname)-8s %(message)s"
    log_levels = {
        'DEBUG': logging.DEBUG,
        'INFO': logging.INFO,
        'WARNING': logging.WARNING,
        'ERROR': logging.ERROR,
    }
    logging.basicConfig(format=FORMAT, level=log_levels.get(level, "INFO"))

async def main():
    set_logging_level(os.getenv('LOG_LEVEL'))
    try:
        alertmanager_url = os.environ["ALERTMANAGER_URL"]
    except KeyError as e:
        logging.error(f"Missing required environment variable: {e.args[0]}")
        return

    # With an Open Messaging webhook the alerts go there, and the Slack app is
    # not started, so its tokens are not needed.
    open_messaging_webhook_url = os.getenv("OPEN_MESSAGING_WEBHOOK_URL")
    if open_messaging_webhook_url:
        logging.info("Posting alerts to Open Messaging")
        receiver = AlertsReceiver(open_messaging=OpenMessaging(open_messaging_webhook_url, alertmanager_url))
    else:
        try:
            slack_bot_token = os.environ["SLACK_BOT_TOKEN"]
            slack_app_token = os.environ["SLACK_APP_TOKEN"]
            slack_app_channel = os.environ["SLACK_APP_CHANNEL"]
        except KeyError as e:
            logging.error(f"Missing required environment variable: {e.args[0]}")
            return

        slack_app = SlackApp(alertmanager_url, slack_bot_token, slack_app_token, slack_app_channel)
        await slack_app.handler.connect_async()
        receiver = AlertsReceiver(slack_app=slack_app)

    await receiver.server.run_task(host='0.0.0.0', port=5001)

if __name__ == "__main__":
    asyncio.run(main())
