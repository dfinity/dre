import json

# Read JSON file
def read_json(file_path):
    with open(file_path, 'r') as file:
        data = json.load(file)
    return data

# Generate Candid-like output from JSON data with multiple subnet_id entries
def generate_candid_output(data):
    output = "( vec {\n"

    for subnet_id, metrics_list in data.items():
        output += "  record {\n"
        output += f'    subnet_id = principal "{subnet_id}";\n'
        output += "    node_metrics_history = vec {\n"
        
        for metrics in metrics_list:
            timestamp_nanos = metrics.get("timestamp_nanos")
            node_metrics = metrics.get("node_metrics", [])
            
            output += "      record {\n"
            output += f"        timestamp_nanos = {timestamp_nanos};\n"
            output += "        node_metrics = vec {\n"
            
            # Add node metrics records, separating them with semicolons
            node_records = []
            for node_metric in node_metrics:
                node_id = node_metric.get("node_id")
                num_blocks_proposed_total = node_metric.get("num_blocks_proposed_total")
                num_block_failures_total = node_metric.get("num_block_failures_total")
                
                # Format each node metric record
                node_record = (
                    "          record {\n"
                    f'            node_id = principal "{node_id}";\n'
                    f"            num_blocks_proposed_total = {num_blocks_proposed_total};\n"
                    f"            num_block_failures_total = {num_block_failures_total};\n"
                    "          }"
                )
                node_records.append(node_record)
            
            # Join node records with semicolons and ensure proper indentation
            output += ";\n".join(node_records) + ";\n"
            output += "        }\n"  # Close the vec for node_metrics
            output += "      };\n"  # Close the record for timestamp and node_metrics
        
        output += "    }\n"  # Close the vec for node_metrics_history
        output += "  };\n"  # Close the record for subnet_id and node_metrics_history

    output += "})\n"  # Close the outer vec

    return output

# Extract node_ids and persist them in a file
def extract_and_save_node_ids(data, file_path):
    node_ids = set()
    
    for metrics_list in data.values():
        for metrics in metrics_list:
            node_metrics = metrics.get("node_metrics", [])
            for node_metric in node_metrics:
                node_id = node_metric.get("node_id")
                if node_id:
                    node_ids.add(node_id)
    
    # Write the node_ids to a file
    with open(file_path, 'w') as file:
        for node_id in node_ids:
            file.write(f"{node_id}\n")
    
    print(f"Node IDs successfully written to {file_path}")

# Write the Candid output to a file
def write_candid(file_path, output):
    with open(file_path, 'w') as file:
        file.write(output)

# Main Function
def main():
    input_file = 'all-backfill-metrics.json'  # Path to your JSON file
    output_file = 'april-backfill.candid'  # Path to your desired output file
    node_ids_file = 'node_ids.txt'  # Path to your node_id file
    
    # Read JSON Data
    json_data = read_json(input_file)
    
    # Generate Candid-like Output
    candid_output = generate_candid_output(json_data)
    
    # Write to output file
    write_candid(output_file, candid_output)
    
    # Extract and save node_ids
    extract_and_save_node_ids(json_data, node_ids_file)

if __name__ == "__main__":
    main()
