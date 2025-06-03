import json
import requests
from datetime import datetime, timedelta, timezone
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
import os
import time

# --- Configuration ---
VICTORIA_METRICS_URL = "https://victoria.mainnet.dfinity.network/select/0/prometheus"  # Example: "http://your-victoria-metrics-instance:8428"
# Path to your registry.json file
INPUT_JSON_PATH = "registry.json"
DAYS_TO_CHART = 60  # Number of past days to include in the chart
OUTPUT_DIR = "provider_charts"  # Directory to save charts


# --- Helper Functions ---

def load_node_provider_data(filepath):
    """
    Loads and parses the JSON file (e.g., registry.json) to map node providers to their nodes.
    Each record in the JSON file is assumed to represent a node operator.
    Handles JSON files that are either a single JSON array or multiple JSON objects per line.
    """
    providers_data = {}
    all_operator_records = []

    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
            # Try parsing as a single JSON array
            try:
                all_operator_records = json.loads(content)[
                    'node_operators']  # Assuming the JSON has a top-level 'operators' key
                if not isinstance(all_operator_records, list):  # If it's a single object, wrap it in a list
                    all_operator_records = [all_operator_records]
            except json.JSONDecodeError:
                # If single array parsing fails, try parsing as JSON objects separated by newlines
                all_operator_records = []
                f.seek(0)  # Reset file pointer
                for line in f:
                    line = line.strip()
                    if line:
                        try:
                            all_operator_records.append(json.loads(line))
                        except json.JSONDecodeError as e_line:
                            print(
                                f"Warning: Skipping line due to JSON decode error: {e_line} - Line: '{line[:100]}...'")
    except FileNotFoundError:
        print(f"Error: Input JSON file not found at {filepath}")
        return None
    except Exception as e:
        print(f"Error reading or parsing JSON file {filepath}: {e}")
        return None

    if not all_operator_records:
        print(f"No records found or parsed from {filepath}")
        return {}

    for operator_record in all_operator_records:
        # Extract node_operator_principal_id for context, though not directly used for grouping here
        # node_operator_id = operator_record.get("node_operator_principal_id", "UnknownOperator")

        provider_id = operator_record.get("node_provider_principal_id")
        if not provider_id:
            # print(f"Warning: Record for operator {node_operator_id} missing node_provider_principal_id. Skipping record: {operator_record}")
            continue

        # Determine provider name (using a generic name if not found)
        node_provider_name = operator_record.get("node_provider_name",  # Common key for provider name
                                                 operator_record.get("computed", {}).get("node_provider_name",
                                                                                         provider_id))

        current_nodes_for_operator = set()
        # Attempt to extract nodes from various possible structures
        if "computed" in operator_record and "nodes_health" in operator_record["computed"]:
            health_data = operator_record["computed"]["nodes_health"]
            if isinstance(health_data, dict):
                for status, node_list in health_data.items():
                    if isinstance(node_list, list):
                        current_nodes_for_operator.update(node_list)
        elif "nodes" in operator_record and isinstance(operator_record["nodes"], list):
            current_nodes_for_operator.update(operator_record["nodes"])
        elif "node_ids" in operator_record and isinstance(operator_record["node_ids"], list):
            current_nodes_for_operator.update(operator_record["node_ids"])
        # Add more checks here if registry.json has other ways of listing nodes for an operator

        if not current_nodes_for_operator:
            pass  # Continue, as a provider might have other operators with nodes

        if provider_id not in providers_data:
            providers_data[provider_id] = {
                "name": node_provider_name,
                "nodes": set()
            }
        providers_data[provider_id]["nodes"].update(current_nodes_for_operator)

    # Convert node sets to lists
    for pid in providers_data:
        providers_data[pid]["nodes"] = list(providers_data[pid]["nodes"])

    return providers_data


def query_victoria_metrics(query, timestamp=None):
    """
    Queries VictoriaMetrics.
    If timestamp is provided, it's an instant query.
    """

    api_endpoint = f"{VICTORIA_METRICS_URL.rstrip('/')}/api/v1/query"
    params = {'query': query}
    if timestamp:
        params['time'] = str(timestamp)

    try:
        response = requests.get(api_endpoint, params=params, timeout=20)
        response.raise_for_status()
        data = response.json()
        if data.get('status') == 'success':
            return data.get('data', {}).get('result', [])
        else:
            print(f"VictoriaMetrics query failed with status: {data.get('status')}, error: {data.get('error')}")
            return None
    except requests.exceptions.RequestException as e:
        print(f"Error querying VictoriaMetrics: {e}")
        return None
    except json.JSONDecodeError as e:
        print(f"Error decoding VictoriaMetrics JSON response: {e}")
        return None


def is_node_assigned(node_id, query_timestamp):
    """
    Checks if a node is assigned to a subnet at a specific timestamp.
    A node is considered assigned if it's 'up' and has the 'ic_subnet' label.
    """
    promql_query = f"up{{ic_node='{node_id}', job='orchestrator'}}"
    time.sleep(0.1)
    result = query_victoria_metrics(promql_query, query_timestamp)

    if result:
        for item in result:
            if 'metric' in item and 'ic_subnet' in item['metric']:
                return True
    return False


def plot_provider_data(provider_id, chart_data, dates_for_plot, output_dir):
    """
    Generates and saves a line chart for a given node provider.
    """
    provider_name = chart_data["name"]
    total_nodes_count = chart_data["total_nodes"]
    assigned_counts_per_day = chart_data["assigned_counts_per_day"]

    if not dates_for_plot:
        print(f"No dates to plot for provider {provider_id}. Skipping chart.")
        return

    if len(dates_for_plot) != len(assigned_counts_per_day):
        print(
            f"Mismatch in length of dates ({len(dates_for_plot)}) and assigned counts ({len(assigned_counts_per_day)}) for provider {provider_id}. Skipping chart.")
        return

    plt.figure(figsize=(12, 6))
    plt.plot(dates_for_plot, [total_nodes_count] * len(dates_for_plot),
             label=f"Total Nodes from Registry ({total_nodes_count})", linestyle='--', color='gray')
    plt.plot(dates_for_plot, assigned_counts_per_day,
             label="Assigned Nodes (via VM query)", marker='o', linestyle='-', color='blue')

    plt.title(f"Node Assignment Over Time for {provider_name} ({provider_id})")
    plt.xlabel("Date")
    plt.ylabel("Number of Nodes")

    plt.gca().xaxis.set_major_formatter(mdates.DateFormatter('%Y-%m-%d'))
    plt.gca().xaxis.set_major_locator(mdates.AutoDateLocator(minticks=5, maxticks=10))
    plt.gcf().autofmt_xdate()

    plt.legend()
    plt.grid(True, linestyle=':', alpha=0.7)
    plt.tight_layout()

    if not os.path.exists(output_dir):
        os.makedirs(output_dir)

    safe_provider_id = "".join(c if c.isalnum() else "_" for c in provider_id)
    filename = os.path.join(output_dir, f"{safe_provider_id}_node_assignment.png")

    try:
        plt.savefig(filename)
        print(f"Chart saved for provider {provider_id} to {filename}")
    except Exception as e:
        print(f"Error saving chart for provider {provider_id}: {e}")
    plt.close()


def main():
    print("Starting node assignment charting script...")


    providers_info = load_node_provider_data(INPUT_JSON_PATH)
    if providers_info is None:
        print(f"Failed to load provider data from {INPUT_JSON_PATH}. Exiting.")
        return
    if not providers_info:
        print(f"No provider data loaded from {INPUT_JSON_PATH}. Exiting.")
        return

    print(f"Loaded data for {len(providers_info)} providers from {INPUT_JSON_PATH}.")

    end_date = datetime.now(timezone.utc).replace(hour=0, minute=0, second=0, microsecond=0)
    start_date = end_date - timedelta(days=DAYS_TO_CHART)

    query_dates = []
    current_day = start_date
    while current_day < end_date:
        query_dates.append(current_day)
        current_day += timedelta(days=1)

    if not query_dates:
        print("No dates to query. Check DAYS_TO_CHART. Exiting.")
        return

    print(f"Querying data from {query_dates[0].strftime('%Y-%m-%d')} to {query_dates[-1].strftime('%Y-%m-%d')}")

    provider_chart_data_aggregated = {}

    for provider_id, info in list(providers_info.items()):  # Limit to first 4 providers for demo
        provider_name = info["name"]
        nodes_for_provider = info["nodes"]
        total_nodes_count = len(nodes_for_provider)

        print(
            f"\nProcessing Provider: {provider_name} ({provider_id}) - Total nodes from registry: {total_nodes_count}")

        assigned_counts_per_day_list = []

        if not nodes_for_provider and total_nodes_count == 0:
            print(f"  Skipping VictoriaMetrics queries for provider {provider_name} as it has no nodes listed.")
            assigned_counts_per_day_list = [0] * len(query_dates)  # Plot zero assigned if no nodes
        else:
            for day_to_query in query_dates:
                query_timestamp = int(datetime(day_to_query.year, day_to_query.month, day_to_query.day,
                                               12, 0, 0, tzinfo=timezone.utc).timestamp())

                assigned_on_this_day = 0

                for i, node_id in enumerate(nodes_for_provider):
                    if is_node_assigned(node_id, query_timestamp):
                        assigned_on_this_day += 1

                assigned_counts_per_day_list.append(assigned_on_this_day)
                print(
                    f"  Assigned nodes on {day_to_query.strftime('%Y-%m-%d')} for {provider_name}: {assigned_on_this_day}/{total_nodes_count}")

        provider_chart_data_aggregated[provider_id] = {
            "name": provider_name,
            "total_nodes": total_nodes_count,
            "assigned_counts_per_day": assigned_counts_per_day_list
        }

    print("\nGenerating charts...")
    if not provider_chart_data_aggregated:
        print("No data aggregated for charting. Exiting.")
        return

    for provider_id, data_for_chart in provider_chart_data_aggregated.items():
        plot_provider_data(provider_id, data_for_chart, query_dates, OUTPUT_DIR)

    print(f"\nScript finished. Charts saved in '{OUTPUT_DIR}' directory.")


if __name__ == "__main__":
    main()
