from flask import Flask, request, jsonify
from slack_app import slack_app

# Flask app
flask_app = Flask(__name__)

@flask_app.route('/alert', methods=['POST'])
def alert():
    data = request.json

    print(data)
    if data:
        for alert in data.get('alerts', []):
            message, blocks = format_alert(alert)
            slack_app.client.chat_postMessage(channel='#test-app', text=message, blocks=blocks)
        return jsonify({"status": "success"}), 200
    else:
        return jsonify({"error": "no data"}), 400

def format_alert(alert):
    proposal_id = alert['labels']['proposal_id']
    alert_name = alert['labels']['alertname']
    alert_severity = alert['labels']['severity']
    alert_description = alert['annotations']['description']
    message = f"*Proposal ID:* {proposal_id}"

    blocks = [
        {
			"type": "section",
			"text": {
				"type": "plain_text",
				"text": "The following proposals will expire before tomorrow at 7pm",
				"emoji": True
			}
		},
		{
			"type": "divider"
		},
		{
			"type": "section",
			"text": {
				"type": "mrkdwn",
				"text": message
			}
		},
        {
			"type": "actions",
			"elements": [
                {
                    "type": "button",
                    "text": {
                        "type": "plain_text",
                        "text": "Vote :judge:",
                        "emoji": True
                    },
                    "value": proposal_id,
                    "action_id": "blackhole",
                    "url": f"https://nns.ic0.app/proposal/?u=qoctq-giaaa-aaaaa-aaaea-cai&proposal={proposal_id}",
                },
                {
                    "type": "button",
                    "text": {
                        "type": "plain_text",
                        "text": "Silence :no_bell:"
                    },
                    "value": proposal_id,
                    "action_id": "silence_alert"
                }
			]
		},
    ]

    return message, blocks

def start_flask_app():
    flask_app.run(port=5000)

def main():
    print("main")
if __name__ == "__main__":
    main()
