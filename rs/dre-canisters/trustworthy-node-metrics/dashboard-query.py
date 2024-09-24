import requests
import json

# Load node IDs from file
with open('nodes_without_np.json', 'r') as file:
    node_ids = json.loads(file.read())  # Assuming node_ids.txt contains a JSON array of node IDs

# API base URL
base_url = 'https://ic-api.internetcomputer.org/api/v3/nodes/'

# List to store not found nodes
not_found_nodes = []

# Iterate over the node IDs and check if they exist
for node_id in node_ids:
    response = requests.get(base_url + node_id)
    
    if response.status_code == 404:  # If the node is not found
        print(f"Node not found: {node_id}")
        not_found_nodes.append(node_id)

# Print the length of the not found nodes
print(f"Number of nodes not found: {len(not_found_nodes)}")

# Save the not found nodes as a JSON file
with open('not_found_nodes.json', 'w') as json_file:
    json.dump(not_found_nodes, json_file, indent=4)

print("Not found nodes saved to 'not_found_nodes.json'")
