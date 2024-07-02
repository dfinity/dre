import os
import sys
sys.path.append(os.path.join(os.path.dirname(__file__)))

from slack_app import SlackConnect
from flask_app import start_flask_app



def main():
    SlackConnect()
    
    # Start Flask app
    start_flask_app()

if __name__ == "__main__":
    main()
