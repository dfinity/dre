import csv
import subprocess

# Define the file paths and constants
csv_file_path = "node_info_api.csv" 
did_file_path = "rs/dre-canisters/trustworthy-node-metrics/src/trustworthy-node-metrics/trustworthy-node-metrics.did"
network = "ic"
canister_name = "trustworthy-node-metrics"

# Function to generate the dfx command for node_metadata
def generate_dfx_command(node_mappings):
    nodes = [
        f'record {{ node_id = principal "{row["node_id"]}"; node_provider_id = principal "{row["node_provider_id"]}"; node_provider_name = "{row["node_provider_name"]}"; }}'
        for row in node_mappings
    ]
    # Prepare the argument for the canister call
    node_metadata_args = f"vec {{ {'; '.join(nodes)} ; }}"
    
    command = [
        "dfx", "canister", "call", canister_name, "node_metadata",
        f'({node_metadata_args})',
        "--candid", did_file_path,
        "--network", network
    ]
    
    return command

# Read the CSV file and filter out missing providers
def read_csv(csv_file_path):
    node_mappings = []
    with open(csv_file_path, mode='r') as file:
        csv_reader = csv.DictReader(file)
        for row in csv_reader:
            if row['node_provider_id'] != "missing":  # Ignore missing providers
                node_mappings.append({
                    "node_id": row["node_id"],
                    "node_provider_id": row["node_provider_id"],
                    "node_provider_name": row["node_provider_name"]
                })
    return node_mappings

# Main execution
if __name__ == "__main__":
    # Read the CSV and get node mappings
    node_mappings = read_csv(csv_file_path)

    if node_mappings:
        # Generate the dfx command
        dfx_command = generate_dfx_command(node_mappings)

        # Execute the dfx command
        try:
            result = subprocess.run(dfx_command, capture_output=True, text=True)
            print(result.stdout)
            if result.stderr:
                print("Error:", result.stderr)
        except Exception as e:
            print("Failed to execute the dfx command:", str(e))
    else:
        print("No valid node mappings found.")
