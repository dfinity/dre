import json
import subprocess
import time
# Define the file paths and constants
json_file_path = "nodes_metadata.json"  # Path to the .json file with node information
did_file_path = "/Users/pietro.di.marco/Documents/dfinity/dre-1/rs/dre-canisters/trustworthy-node-metrics/src/trustworthy-node-metrics/trustworthy-node-metrics.did"
network = "ic"
canister_name = "trustworthy-node-metrics"

# Function to generate the dfx command for a single node
def generate_dfx_command(node_data):
    command = [
        "dfx", "canister", "call", canister_name, "nodes_metadata_backfill",
        f'(record {{ node_id = principal "{node_data["node_id"]}"; node_operator_id = principal "{node_data["node_operator_id"]}"; '
        f'node_provider_id = principal "{node_data["node_provider_id"]}"; node_provider_name = "{node_data["node_provider_name"]}"; '
        f'dc_id = "{node_data["dc_id"]}"; region = "{node_data["region"]}"; node_type = "{node_data["node_type"]}"; }})',
        "--candid", did_file_path,
        "--network", network
    ]
    return command

# Read the .json file and extract node information
def read_json(json_file_path):
    with open(json_file_path, mode='r') as file:
        # Each line is a separate JSON object, so we use a list to store them
        node_mappings = [json.loads(line) for line in file]
    return node_mappings

# Execute the dfx command for each node
def execute_dfx_command(command):
    try:
        result = subprocess.run(command, capture_output=True, text=True)
        print("Command executed successfully:", result.stdout)
        if result.stderr:
            print("Error:", result.stderr)
    except Exception as e:
        print("Failed to execute the dfx command:", str(e))

# Main execution
if __name__ == "__main__":
    # Read the JSON file and get node mappings
    node_mappings = read_json(json_file_path)

    if node_mappings:
        # Loop through each node and execute a separate command
        for node in node_mappings:
            # Generate the dfx command for the current node
            dfx_command = generate_dfx_command(node)
            
            # Execute the dfx command for the current node
            execute_dfx_command(dfx_command)
    else:
        print("No valid node mappings found.")
