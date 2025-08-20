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

def read_csv_rewards(result_dir):
    """Read total_adjusted_rewards_xdr_permyriad from each node provider's CSV files and sum them."""
    csv_rewards = {}
    
    # Iterate through each node provider directory
    for provider_dir in os.listdir(result_dir):
        provider_path = os.path.join(result_dir, provider_dir)
        
        # Skip if not a directory or if it's the CSV file
        if not os.path.isdir(provider_path) or provider_dir.endswith('.csv'):
            continue
        
        csv_file = os.path.join(provider_path, 'overall_rewards_per_day.csv')
        
        if os.path.exists(csv_file):
            total_rewards = 0
            
            with open(csv_file, 'r') as f:
                reader = csv.DictReader(f)
                for row in reader:
                    # Skip empty rows
                    if 'total_adjusted_rewards_xdr_permyriad' in row and row['total_adjusted_rewards_xdr_permyriad']:
                        total_rewards += float(row['total_adjusted_rewards_xdr_permyriad'])
            
            csv_rewards[provider_dir] = total_rewards
        else:
            print(f"Warning: CSV file not found for provider {provider_dir}")
    
    return csv_rewards

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

def compare_rewards(json_rewards, csv_rewards, type1_percentages, xdr_to_icp_rate):
    """Compare rewards from JSON and CSV and print results."""
    print("\n" + "=" * 130)
    print("REWARD COMPARISON: July 2025")
    print("Last Rewards = Actual rewards received (production algorithm)")
    print("Perf Expected = Performance Based expected rewards (new algorithm)")
    print("Last Rwd 30d = Last rewards adjusted for 30 days (JSON - 1.437% of JSON)")
    print("All amounts in XDR permyriad and ICP")
    print("Difference: + means new algorithm gives MORE, - means LESS")
    print("=" * 130)
    print(f"{'Provider':<15} {'Last Rewards':<15} {'Perf Expected':<15} {'Last Rwd 30d':<15} {'Diff XDR':<15} {'Diff %':<10} {'Diff ICP':<15} {'Type1 %':<10} {'Status':<10}")
    print("=" * 130)
    
    all_providers = set(json_rewards.keys()) | set(csv_rewards.keys())
    
    for provider in sorted(all_providers):
        json_reward = json_rewards.get(provider, 0.0)
        csv_reward = csv_rewards.get(provider, 0.0)
        json_30day = json_reward * (1 - 0.01437371663)  # Adjust JSON for 30 days
        difference = csv_reward - json_30day  # Positive if new algo gives more than adjusted JSON, negative if less
        icp_difference = difference / xdr_to_icp_rate if xdr_to_icp_rate else 0.0  # Convert to ICP
        type1_pct = type1_percentages.get(provider, 0.0)
        
        # Calculate percentage difference relative to Last Rwd 30d
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
        
        print(f"{truncate_provider_id(provider):<15} {json_reward:<15.0f} {csv_reward:<15.0f} {json_30day:<15.0f} {difference:<+15.0f} {diff_percentage:<+10.1f}% {icp_difference:<+15.2f} {type1_pct:<10.1f} {status:<10}")
    
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
        discrepancies = []
        for provider in both:
            json_reward = json_rewards[provider]
            csv_reward = csv_rewards[provider]
            json_30day = json_reward * (1 - 0.01437371663)  # 30-day adjustment
            diff = csv_reward - json_30day  # Signed difference using adjusted JSON values
            if diff != 0:  # Only show non-zero differences
                discrepancies.append((provider, json_reward, csv_reward, json_30day, diff))
        
        discrepancies.sort(key=lambda x: x[4], reverse=True)  # Sort by difference (index 4)
        
        print(f"\nALL DISCREPANCIES (providers in both datasets, ordered by difference):")
        print(f"{'Provider':<15} {'Last Rewards':<15} {'Perf Expected':<15} {'Last Rwd 30d':<15} {'Diff XDR':<15} {'Diff %':<10} {'Diff ICP':<15} {'Type1 %':<10}")
        print("-" * 130)
        for provider, json_reward, csv_reward, json_30day, diff in discrepancies:
            icp_diff = diff / xdr_to_icp_rate if xdr_to_icp_rate else 0.0
            type1_pct = type1_percentages.get(provider, 0.0)
            
            # Calculate percentage difference relative to Last Rwd 30d
            if json_30day != 0:
                diff_percentage = (diff / json_30day) * 100
            else:
                diff_percentage = 0.0
            
            print(f"{truncate_provider_id(provider):<15} {json_reward:<15.0f} {csv_reward:<15.0f} {json_30day:<15.0f} {diff:<+15.0f} {diff_percentage:<+10.1f}% {icp_diff:<+15.2f} {type1_pct:<10.1f}")

def main():
    # Paths
    script_dir = Path(__file__).parent
    json_path = script_dir / "last_rewards_received.json"
    result_dir = script_dir / "result"
    
    print("Loading JSON data...")
    reward_data = load_last_rewards_received(json_path)
    
    print("Computing XDR rewards from JSON...")
    json_rewards = compute_xdr_rewards_from_json(reward_data)
    xdr_to_icp_rate = get_xdr_to_icp_conversion_rate(reward_data)
    
    print("Reading CSV rewards...")
    csv_rewards = read_csv_rewards(result_dir)
    
    print("Calculating Type1 percentages...")
    type1_percentages = get_type1_percentages(result_dir)
    
    print("Comparing rewards...\n")
    compare_rewards(json_rewards, csv_rewards, type1_percentages, xdr_to_icp_rate)

if __name__ == "__main__":
    main()
