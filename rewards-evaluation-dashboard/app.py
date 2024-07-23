from flask import Flask, render_template, request, jsonify
import json
from ic import Client
from ic.agent import Agent
from ic.identity import Identity

app = Flask(__name__)

@app.route('/')
def index():
    nodes = get_nodes()
    return render_template('index.html', nodes=nodes)

def get_nodes():
    # Replace with actual identity and canister URL
    client = Client("https://ic0.app");
    identity = Identity(anonymous=True)
    agent = Agent(identity, client)
    response = agent.query_endpoint('oqi72-gaaaa-aaaam-ac2pq-cai', "get_nodes")
    print(response)
    return response['nodes']

@app.route('/get_data', methods=['POST'])
def get_data():
    node_id = request.form.get('node_id')
    data = query_node_data(node_id)
    return jsonify(data)


def query_node_data(node_id):
    # Replace with actual identity and canister URL
    identity = Identity.from_pem_file('identity.pem')
    agent = Agent(identity, 'https://ic0.app')
    canister = agent.get_canister('your_canister_id')

    # Query the canister with the selected node_id
    response = canister.query("get_data_by_node", {'node_id': node_id})
    return response['data']

if __name__ == '__main__':
    app.run(debug=True)
