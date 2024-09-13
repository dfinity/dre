import json
import csv

# Load node_ids.txt
def load_node_ids(file_path):
    with open(file_path, 'r') as file:
        return file.read().splitlines()

# Load registry_dump.json
def load_registry_dump(file_path):
    with open(file_path, 'r') as file:
        return json.load(file)

# Extract node information from registry_dump.json
def extract_node_info(registry_dump):
    node_info = {}
    for entry in registry_dump['subnets']:
        if 'membership' in entry: 
            for node_id in entry['membership']:
                node_info[node_id] = {}
                node_info[node_id]['subnet_id'] = entry['subnet_id']
    for entry in registry_dump['node_operators']:
        if 'nodes_health' in entry:
            healthy_nodes = entry['nodes_health'].get('Healthy', [])
            degraded_nodes = entry['nodes_health'].get('Degraded', [])
            all_nodes = healthy_nodes + degraded_nodes
            
            for node_id in all_nodes:
                if node_id not in node_info.keys():
                    node_info[node_id] = {}
                node_info[node_id]['node_provider_principal_id'] = entry['node_provider_principal_id']
                node_info[node_id]['node_provider_name'] = entry['node_provider_name']
    return node_info

# Write the output to CSV
def write_to_csv(node_ids, node_info, output_file):
    with open(output_file, 'w', newline='') as csvfile:
        fieldnames = ['node_id', 'node_provider_principal_id', 'node_provider_name', 'subnet_id', 'status']
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
        
        writer.writeheader()

        for node_id in node_ids:
            if node_id in node_info:
                node_data = node_info[node_id]
                writer.writerow({
                    'node_id': node_id,
                    'node_provider_principal_id': node_data.get('node_provider_principal_id', 'missing'),
                    'node_provider_name': node_data.get('node_provider_name', 'missing'),
                    'subnet_id': node_data.get('subnet_id', 'missing'),
                    'status': 'found'
                })
            else:
                writer.writerow({
                    'node_id': node_id,
                    'node_provider_principal_id': 'missing',
                    'node_provider_name': 'missing',
                    'subnet_id': 'missing',
                    'status': 'not found'
                })

if __name__ == "__main__":
    # Paths to the input files
    node_ids_file = '/Users/pietro.di.marco/Documents/dfinity/dre-1/rs/dre-canisters/trustworthy-node-metrics/src/trustworthy-node-metrics/node_ids.txt'
    registry_dump_file = '/Users/pietro.di.marco/Documents/dfinity/dre/rs/cli/registry_dump.json'
    output_csv_file = 'node_info.csv'
    
    # Load data
    node_ids = load_node_ids(node_ids_file)
    registry_dump = load_registry_dump(registry_dump_file)

    # Extract node information
    node_info = extract_node_info(registry_dump)

    

    # Write the results to CSV
    write_to_csv(node_ids, node_info, output_csv_file)
    
    print(f"CSV file generated: {output_csv_file}")
