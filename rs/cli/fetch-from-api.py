import requests
import csv
import time

# Load node_ids.txt
def load_node_ids(file_path):
    with open(file_path, 'r') as file:
        return file.read().splitlines()

# Fetch node information from the API
def fetch_node_info(node_id):
    url = f'https://ic-api.internetcomputer.org/api/v3/nodes/{node_id}'
    headers = {'accept': 'application/json'}
    
    try:
        response = requests.get(url, headers=headers)
        if response.status_code == 200:
            return response.json()
        else:
            print(f"Error: Unable to fetch data for node_id {node_id}. Status code: {response.status_code}")
            return None
    except Exception as e:
        print(f"Error: {e}")
        return None

# Write the output to CSV
def write_to_csv(node_ids, output_file):
    with open(output_file, 'w', newline='') as csvfile:
        fieldnames = ['node_id', 'node_provider_id', 'node_provider_name', 'subnet_id', 'status']
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
        
        writer.writeheader()

        for node_id in node_ids:
            node_data = fetch_node_info(node_id)
            if node_data:
                writer.writerow({
                    'node_id': node_id,
                    'node_provider_id': node_data.get('node_provider_id', 'missing'),
                    'node_provider_name': node_data.get('node_provider_name', 'missing'),
                    'subnet_id': node_data.get('subnet_id', 'missing'),
                    'status': 'found'
                })
            else:
                writer.writerow({
                    'node_id': node_id,
                    'node_provider_id': 'missing',
                    'node_provider_name': 'missing',
                    'subnet_id': 'missing',
                    'status': 'not found'
                })
            # Sleep for a short while to avoid overwhelming the API with requests
            time.sleep(1)  # 1-second delay between requests

if __name__ == "__main__":
    # Paths to the input file
    node_ids_file = '/Users/pietro.di.marco/Documents/dfinity/dre-1/rs/dre-canisters/trustworthy-node-metrics/src/trustworthy-node-metrics/node_ids.txt'
    output_csv_file = 'node_info_api.csv'
    
    # Load node IDs
    node_ids = load_node_ids(node_ids_file)

    # Write the results to CSV by fetching the data via API
    write_to_csv(node_ids, output_csv_file)
    
    print(f"CSV file generated: {output_csv_file}")
