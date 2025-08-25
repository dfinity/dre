#!/usr/bin/env python3

import json
import csv
import os
from pathlib import Path

def load_last_rewards_received(json_path):
    """Load and parse the last_rewards_received.json file."""
    with open(json_path, 'r') as f:
        data = json.load(f)
    return data['data']

def get_xdr_to_icp_conversion_rate(reward_data):
    """Extract the XDR to ICP conversion rate from the reward data."""
    if reward_data:
        return reward_data[0]['xdr_conversion_rate']['xdr_permyriad_per_icp']
    return None

def truncate_provider_id(provider_id):
    """Truncate provider ID at the first dash."""
    return provider_id.split('-')[0] if provider_id else provider_id

def compute_xdr_rewards_from_json(reward_data):
    """Compute XDR rewards from JSON data using the formula: amount_e8s/100_000_000 * xdr_permyriad_per_icp"""
    rewards = {}
    
    for entry in reward_data:
        node_provider = entry['node_provider']
        amount_e8s = entry['amount_e8s']
        xdr_permyriad_per_icp = entry['xdr_conversion_rate']['xdr_permyriad_per_icp']
        
        # Formula: amount_e8s/100_000_000 * xdr_permyriad_per_icp
        xdr_reward = (amount_e8s / 100_000_000) * xdr_permyriad_per_icp
        rewards[node_provider] = xdr_reward
    
    return rewards

def read_csv_rewards_and_nodes(result_dir):
    """Read total_adjusted_rewards_xdr_permyriad, nodes_assigned, and nodes_in_registry from each node provider's CSV files."""
    csv_rewards = {}
    csv_nodes_assigned = {}
    csv_nodes_in_registry = {}
    
    # Iterate through each node provider directory
    for provider_dir in os.listdir(result_dir):
        provider_path = os.path.join(result_dir, provider_dir)
        
        # Skip if not a directory or if it's the CSV file
        if not os.path.isdir(provider_path) or provider_dir.endswith('.csv'):
            continue
        
        csv_file = os.path.join(provider_path, 'overall_rewards_per_day.csv')
        
        if os.path.exists(csv_file):
            total_rewards = 0
            daily_nodes_assigned = []
            daily_nodes_in_registry = []
            
            with open(csv_file, 'r') as f:
                reader = csv.DictReader(f)
                for row in reader:
                    # Skip empty rows
                    if 'total_adjusted_rewards_xdr_permyriad' in row and row['total_adjusted_rewards_xdr_permyriad']:
                        total_rewards += float(row['total_adjusted_rewards_xdr_permyriad'])
                    
                    if 'nodes_assigned' in row and row['nodes_assigned']:
                        try:
                            daily_nodes_assigned.append(int(row['nodes_assigned']))
                        except ValueError:
                            continue
                    
                    if 'nodes_in_registry' in row and row['nodes_in_registry']:
                        try:
                            daily_nodes_in_registry.append(int(row['nodes_in_registry']))
                        except ValueError:
                            continue
            
            csv_rewards[provider_dir] = total_rewards
            csv_nodes_assigned[provider_dir] = daily_nodes_assigned
            csv_nodes_in_registry[provider_dir] = daily_nodes_in_registry
        else:
            print(f"Warning: CSV file not found for provider {provider_dir}")
    
    return csv_rewards, csv_nodes_assigned, csv_nodes_in_registry

def get_type1_percentages(result_dir):
    """Calculate the percentage of Type1 nodes for each node provider."""
    type1_percentages = {}
    
    # Iterate through each node provider directory
    for provider_dir in os.listdir(result_dir):
        provider_path = os.path.join(result_dir, provider_dir)
        
        # Skip if not a directory or if it's the CSV file
        if not os.path.isdir(provider_path) or provider_dir.endswith('.csv'):
            continue
        
        nodes_file = os.path.join(provider_path, 'nodes_results.csv')
        
        if os.path.exists(nodes_file):
            unique_nodes = {}  # Track unique nodes and their types
            
            with open(nodes_file, 'r') as f:
                reader = csv.DictReader(f)
                for row in reader:
                    if 'node_id' in row and 'node_reward_type' in row:
                        node_id = row['node_id']
                        node_type = row['node_reward_type']
                        # Store the first occurrence of each node (they repeat per day)
                        if node_id not in unique_nodes:
                            unique_nodes[node_id] = node_type
            
            if unique_nodes:
                total_nodes = len(unique_nodes)
                type1_nodes = sum(1 for node_type in unique_nodes.values() if node_type == 'Type1')
                type1_percentage = (type1_nodes / total_nodes) * 100 if total_nodes > 0 else 0
                type1_percentages[provider_dir] = type1_percentage
            else:
                type1_percentages[provider_dir] = 0
        else:
            type1_percentages[provider_dir] = 0
    
    return type1_percentages

def get_providers_with_zero_nodes_assigned_and_large_registry(result_dir):
    """Identify node providers that have at least one day with nodes_assigned == 0 AND more than 3 nodes_in_registry every day."""
    zero_nodes_large_registry_providers = {}
    
    # Iterate through each node provider directory
    for provider_dir in os.listdir(result_dir):
        provider_path = os.path.join(result_dir, provider_dir)
        
        # Skip if not a directory or if it's the CSV file
        if not os.path.isdir(provider_path) or provider_dir.endswith('.csv'):
            continue
        
        csv_file = os.path.join(provider_path, 'overall_rewards_per_day.csv')
        
        if os.path.exists(csv_file):
            zero_days = []
            daily_nodes_in_registry = []
            
            with open(csv_file, 'r') as f:
                reader = csv.DictReader(f)
                
                for row in reader:
                    if 'nodes_assigned' in row and 'day' in row:
                        try:
                            nodes_assigned = int(row['nodes_assigned'])
                            date = row['day']
                            if nodes_assigned == 0:
                                zero_days.append(date)
                        except (ValueError, KeyError):
                            continue
                    
                    if 'nodes_in_registry' in row and row['nodes_in_registry']:
                        try:
                            daily_nodes_in_registry.append(int(row['nodes_in_registry']))
                        except ValueError:
                            continue
            
            # Only include if provider has zero nodes on at least one day AND more than 3 nodes_in_registry every day
            if zero_days and daily_nodes_in_registry and all(nodes > 3 for nodes in daily_nodes_in_registry):
                zero_nodes_large_registry_providers[provider_dir] = zero_days
    
    return zero_nodes_large_registry_providers



def compare_rewards(json_rewards, csv_rewards, type1_percentages, xdr_to_icp_rate, zero_nodes_providers, csv_nodes_assigned, csv_nodes_in_registry):
    """Compare rewards from JSON and CSV and print results."""
    print("\n" + "=" * 130)
    print("REWARD COMPARISON: July 2025")
    print("Last Rewards = Actual rewards received (production algorithm)")
    print("Perf Expected = Performance Based expected rewards (new algorithm)")
    print("Last Rewards 30d = Last rewards adjusted for 30 days (JSON - 1.437% of JSON)")
    print("All amounts in XDR permyriad and ICP")
    print("Difference: + means new algorithm gives MORE, - means LESS")
    print("=" * 130)
    print(f"{'Provider':<15} {'Last Rewards':<15} {'Perf Expected':<15} {'Last Rewards 30d':<15} {'Diff XDR':<15} {'Diff %':<10} {'Diff ICP':<15} {'Status':<10}")
    print("=" * 130)
    
    all_providers = set(json_rewards.keys()) | set(csv_rewards.keys())
    
    for provider in sorted(all_providers):
        json_reward = json_rewards.get(provider, 0.0)
        csv_reward = csv_rewards.get(provider, 0.0)
        json_30day = json_reward * (1 - 0.01437371663)  # Adjust JSON for 30 days
        difference = csv_reward - json_30day  # Positive if new algo gives more than adjusted JSON, negative if less
        icp_difference = difference / xdr_to_icp_rate if xdr_to_icp_rate else 0.0  # Convert to ICP
        
        # Calculate percentage difference relative to Last Rewards 30d
        if json_30day != 0:
            diff_percentage = (difference / json_30day) * 100
        else:
            diff_percentage = 0.0
        
        # Determine status
        if provider in json_rewards and provider in csv_rewards:
            status = "Both"
        elif provider in json_rewards:
            status = "LastRwd only"
        else:
            status = "PB only"
        
        print(f"{truncate_provider_id(provider):<15} {json_reward:<15.0f} {csv_reward:<15.0f} {json_30day:<15.0f} {difference:<+15.0f} {diff_percentage:<+10.1f}% {icp_difference:<+15.2f} {status:<10}")
    
    print("=" * 130)
    
    # Detailed analysis
    json_only = set(json_rewards.keys()) - set(csv_rewards.keys())
    csv_only = set(csv_rewards.keys()) - set(json_rewards.keys())
    both = set(json_rewards.keys()) & set(csv_rewards.keys())
    
    # Summary statistics
    total_json = sum(json_rewards.values())
    total_csv = sum(csv_rewards.values())
    total_json_30day = total_json * (1 - 0.01437371663)  # 30-day adjustment
    total_difference = total_csv - total_json_30day  # Signed difference using adjusted JSON values
    total_icp_difference = total_difference / xdr_to_icp_rate if xdr_to_icp_rate else 0.0
    
    print(f"\nSUMMARY:")
    print(f"XDR to ICP conversion rate:                    {xdr_to_icp_rate} XDR permyriad per ICP")
    print(f"Total Last Rewards Received (Production):      {total_json:,.0f} XDR permyriad")
    print(f"Total Performance Based Expected:              {total_csv:,.0f} XDR permyriad")
    print(f"Total Last Rewards 30-day (Adjusted):          {total_json_30day:,.0f} XDR permyriad")
    print(f"Total Difference (Perf vs Last 30d):           {total_difference:+,.0f} XDR permyriad")
    print(f"Total Difference (ICP):                       {total_icp_difference:+,.2f} ICP")
    print(f"Percentage Difference:                        {(total_difference/total_json_30day*100):+.2f}%")
    
    print(f"\nPROVIDER COVERAGE:")
    print(f"Providers in Last Rewards (got rewards):         {len(json_rewards)}")
    print(f"Providers in Performance Expected:               {len(csv_rewards)}")
    print(f"Providers in both datasets:                      {len(both)}")
    print(f"Providers only in Last Rewards (production):     {len(json_only)}")
    print(f"Providers only in Performance (new algorithm):   {len(csv_only)}")
    
    if json_only:
        print(f"\nPROVIDERS ONLY IN LAST REWARDS (got 0 XDR):")
        for provider in sorted(json_only):
            reward = json_rewards[provider]
            print(f"  {truncate_provider_id(provider)}: {reward:,.0f} XDR permyriad")
    
    if csv_only:
        print(f"\nPROVIDERS ONLY IN PERFORMANCE EXPECTED (expected rewards but didn't receive):")
        for provider in sorted(csv_only):
            reward = csv_rewards[provider]
            print(f"  {truncate_provider_id(provider)}: {reward:,.0f} XDR permyriad")
    
    # Show biggest discrepancies for providers in both datasets
    if both:
        # Prepare discrepancies data
        discrepancies = []
        for provider in both:
            json_reward = json_rewards[provider]
            csv_reward = csv_rewards[provider]
            json_30day = json_reward * (1 - 0.01437371663)  # 30-day adjustment
            diff = csv_reward - json_30day  # Signed difference using adjusted JSON values
            if diff != 0:  # Only show non-zero differences
                discrepancies.append((provider, json_reward, csv_reward, json_30day, diff))
        
        # Sort by percentage difference in descending order
        discrepancies_with_percentage = []
        for provider, json_reward, csv_reward, json_30day, diff in discrepancies:
            if json_30day != 0:
                diff_percentage = (diff / json_30day) * 100
            else:
                diff_percentage = 100
            discrepancies_with_percentage.append((provider, json_reward, csv_reward, json_30day, diff, diff_percentage))
        
        discrepancies_with_percentage.sort(key=lambda x: x[5], reverse=True)  # Sort by percentage difference (index 5)
        
        # Display all discrepancies
        if discrepancies_with_percentage:
            print(f"\nALL DISCREPANCIES (providers with non-zero differences):")
            print(f"{'Provider':<15} {'Last Rewards':<15} {'Perf Expected':<15} {'Last Rewards 30d':<15} {'Diff XDR':<15} {'Diff %':<10} {'Diff ICP':<15}")
            print("-" * 130)
            for provider, json_reward, csv_reward, json_30day, diff, diff_percentage in discrepancies_with_percentage:
                icp_diff = diff / xdr_to_icp_rate if xdr_to_icp_rate else 0.0
                
                print(f"{truncate_provider_id(provider):<15} {json_reward:<15.0f} {csv_reward:<15.0f} {json_30day:<15.0f} {diff:<+15.0f} {diff_percentage:<+10.1f}% {icp_diff:<+15.2f}")
        else:
            print(f"\nALL DISCREPANCIES: No discrepancies found")
        
        # Summary
        print(f"\n" + "=" * 80)
        print("DISCREPANCIES SUMMARY")
        print("=" * 80)
        print(f"Total providers with discrepancies: {len(discrepancies_with_percentage)}")
        print(f"Note: Only providers with non-zero differences are shown")
        print("=" * 80)

def calculate_three_incremental_differences(json_rewards, no_penalties_rewards, with_penalties_rewards, xdr_to_icp_rate):
    """Calculate and display the three incremental differences for each provider."""
    print("\n" + "=" * 180)
    print("THREE INCREMENTAL DIFFERENCES ANALYSIS")
    print("=" * 180)
    print("30dInc: 30-day adjustment (JSON - 1.437% of JSON)")
    print("AlgoInc: New algorithm without penalties (no_penalties vs json_30day)")
    print("PenaltyInc: Penalties applied (with_penalties vs no_penalties)")
    print("=" * 180)
    
    print(f"{'Provider':<15} {'JSON':<12} {'JSON_30d':<12} {'No_Penal':<12} {'With_Penal':<12} {'30dInc%':<8} {'AlgoInc%':<8} {'PenaltyInc%':<8} {'Total%':<8} {'Total_ICP':<12}")
    print("-" * 180)
    
    all_providers = set(json_rewards.keys()) | set(no_penalties_rewards.keys()) | set(with_penalties_rewards.keys())
    
    # Sort providers by total percentage difference
    incremental_results = []
    for provider in all_providers:
        json_reward = json_rewards.get(provider, 0.0)
        no_penalties_reward = no_penalties_rewards.get(provider, 0.0)
        with_penalties_reward = with_penalties_rewards.get(provider, 0.0)
        
        # Increment 1: 30-day adjustment
        json_30day = json_reward * (1 - 0.01437371663)
        increment1_percentage = ((json_30day - json_reward) / json_reward) * 100 if json_reward != 0 else 0
        
        # Increment 2: New algorithm without penalties
        increment2_percentage = ((no_penalties_reward - json_30day) / json_30day) * 100 if json_30day != 0 else 0
        
        # Increment 3: Penalties applied
        increment3_percentage = ((with_penalties_reward - no_penalties_reward) / no_penalties_reward) * 100 if no_penalties_reward != 0 else 0
        
        # Cumulative total
        total_diff = with_penalties_reward - json_reward
        total_percentage = (total_diff / json_reward) * 100 if json_reward != 0 else 0
        
        incremental_results.append((provider, json_reward, json_30day, no_penalties_reward, with_penalties_reward,
                                  increment1_percentage, increment2_percentage, increment3_percentage, total_percentage))
    
    # Sort by total percentage difference
    incremental_results.sort(key=lambda x: x[8], reverse=True)
    
    for provider, json_reward, json_30day, no_penalties_reward, with_penalties_reward, inc1_pct, inc2_pct, inc3_pct, total_pct in incremental_results:
        provider_short = truncate_provider_id(provider)
        total_icp = (with_penalties_reward - json_reward) / xdr_to_icp_rate if xdr_to_icp_rate else 0.0
        
        print(f"{provider_short:<15} {json_reward:<12.0f} {json_30day:<12.0f} "
              f"{no_penalties_reward:<12.0f} {with_penalties_reward:<12.0f} "
              f"{inc1_pct:<+8.1f} {inc2_pct:<+8.1f} "
              f"{inc3_pct:<+8.1f} {total_pct:<+8.1f} "
              f"{total_icp:<+12.2f}")
    
    print("=" * 180)
    
    # Summary statistics
    total_json = sum(data[1] for data in incremental_results)
    total_no_penalties = sum(data[3] for data in incremental_results)
    total_with_penalties = sum(data[4] for data in incremental_results)
    
    print(f"\nSUMMARY STATISTICS:")
    print(f"Total JSON rewards: {total_json:,.0f} XDR permyriad")
    print(f"Total no penalties: {total_no_penalties:,.0f} XDR permyriad")
    print(f"Total with penalties: {total_with_penalties:,.0f} XDR permyriad")
    
    # Calculate overall increments
    total_inc1 = total_no_penalties - total_json
    total_inc2 = total_with_penalties - total_no_penalties
    total_inc3 = total_with_penalties - total_json
    
    print(f"\nOVERALL INCREMENTS:")
    print(f"30dInc (30-day adjustment): {total_inc1:+,.0f} XDR permyriad")
    print(f"AlgoInc (new algorithm): {total_inc2:+,.0f} XDR permyriad")
    print(f"Total change: {total_inc3:+,.0f} XDR permyriad")

def main():
    # Paths
    script_dir = Path(__file__).parent
    json_path = script_dir / "last_rewards_received.json"
    result_dir = "/Users/pietro.di.marco/Documents/dfinity/082025_results/082025_results_with_penalties/rewards_results/result"
    no_penalties_dir = "/Users/pietro.di.marco/Documents/dfinity/082025_results/082025_results_no_penalties/rewards_results/result"
    
    print("Loading JSON data...")
    reward_data = load_last_rewards_received(json_path)
    
    print("Computing XDR rewards from JSON...")
    json_rewards = compute_xdr_rewards_from_json(reward_data)
    xdr_to_icp_rate = get_xdr_to_icp_conversion_rate(reward_data)
    
    print("Reading CSV rewards and nodes data...")
    csv_rewards, csv_nodes_assigned, csv_nodes_in_registry = read_csv_rewards_and_nodes(result_dir)
    
    print("Reading no penalties CSV rewards...")
    no_penalties_rewards, _, _ = read_csv_rewards_and_nodes(no_penalties_dir)
    
    print("Calculating Type1 percentages...")
    type1_percentages = get_type1_percentages(result_dir)
    
    print("Identifying providers with zero nodes assigned and large registry...")
    zero_nodes_providers = get_providers_with_zero_nodes_assigned_and_large_registry(result_dir)
    
    print("Comparing rewards...\n")
    compare_rewards(json_rewards, csv_rewards, type1_percentages, xdr_to_icp_rate, zero_nodes_providers, csv_nodes_assigned, csv_nodes_in_registry)
    
    print("\n" + "=" * 60)
    print("THREE INCREMENTAL DIFFERENCES ANALYSIS")
    print("=" * 60)
    calculate_three_incremental_differences(json_rewards, no_penalties_rewards, csv_rewards, xdr_to_icp_rate)

if __name__ == "__main__":
    main()
