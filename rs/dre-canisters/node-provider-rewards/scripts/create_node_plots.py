#!/usr/bin/env python3
"""
Create line plots for each node provider showing relative failure rates over time.
One single plot per provider showing all nodes displayed as different colored lines.
Processes all providers equally without categorization.
"""

import os
import csv
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
from datetime import datetime
from pathlib import Path
import pandas as pd
from collections import defaultdict
import math
import numpy as np
import json

def truncate_provider_id(provider_id):
    """Truncate provider ID to just the part before the first dash."""
    return provider_id.split('-')[0]

def parse_date(date_str):
    """Parse date string in DD-MM-YYYY format to datetime object."""
    return datetime.strptime(date_str, "%d-%m-%Y")

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

def load_last_rewards_received(json_path):
    """Load and parse the last_rewards_received.json file."""
    with open(json_path, 'r') as f:
        data = json.load(f)
    return data['data']

def compute_xdr_rewards_from_json(reward_data):
    """Compute XDR rewards from JSON data."""
    rewards = {}
    
    for entry in reward_data:
        node_provider = entry['node_provider']
        amount_e8s = entry['amount_e8s']
        xdr_permyriad_per_icp = entry['xdr_conversion_rate']['xdr_permyriad_per_icp']
        
        # Formula: amount_e8s/100_000_000 * xdr_permyriad_per_icp
        xdr_reward = (amount_e8s / 100_000_000) * xdr_permyriad_per_icp
        rewards[node_provider] = xdr_reward
    
    return rewards

def calculate_three_incremental_differences(provider_name, json_rewards, no_penalties_rewards, with_penalties_rewards):
    """Calculate the three incremental differences for a specific provider."""
    json_reward = json_rewards.get(provider_name, 0.0)
    no_penalties_reward = no_penalties_rewards.get(provider_name, 0.0)
    with_penalties_reward = with_penalties_rewards.get(provider_name, 0.0)
    
    # Increment 1: 30-day adjustment
    json_30day = json_reward * (1 - 0.01437371663)
    increment1_percentage = ((json_30day - json_reward) / json_reward) * 100 if json_reward != 0 else 0
    
    # Increment 2: New algorithm without penalties
    increment2_percentage = ((no_penalties_reward - json_30day) / json_30day) * 100 if json_30day != 0 else 0
    
    # Increment 3: Penalties applied
    increment3_percentage = ((with_penalties_reward - no_penalties_reward) / no_penalties_reward) * 100 if no_penalties_reward != 0 else 0
    
    # Total difference (Increment 2 + Increment 3)
    total_diff = with_penalties_reward - json_reward
    total_percentage = (total_diff / json_reward) * 100 if json_reward != 0 else 0
    
    return {
        'increment1': increment1_percentage,
        'increment2': increment2_percentage,
        'increment3': increment3_percentage,
        'total': total_percentage,
        'json_reward': json_reward,
        'json_30day': json_30day,
        'no_penalties_reward': no_penalties_reward,
        'with_penalties_reward': with_penalties_reward
    }



def create_provider_node_plots(provider_dir, output_dir, json_rewards, no_penalties_rewards, with_penalties_rewards):
    """Create a single plot per provider showing relative failure rates for all nodes over time."""
    provider_name = os.path.basename(provider_dir)
    provider_short = truncate_provider_id(provider_name)
    
    # Calculate three incremental differences
    incremental_data = calculate_three_incremental_differences(provider_name, json_rewards, no_penalties_rewards, with_penalties_rewards)
    
    nodes_csv_path = os.path.join(provider_dir, "nodes_results.csv")
    
    if not os.path.exists(nodes_csv_path):
        print(f"Warning: nodes_results.csv not found for provider {provider_short}")
        return
    
    print(f"Processing provider: {provider_short} (Total Diff: {incremental_data['total']:+.1f}%, 30dInc: {incremental_data['increment1']:+.1f}%, AlgoInc: {incremental_data['increment2']:+.1f}%, PenaltyInc: {incremental_data['increment3']:+.1f}%)")
    
    # Read the CSV data
    node_data = defaultdict(list)
    all_dates = set()
    
    try:
        with open(nodes_csv_path, 'r') as csvfile:
            reader = csv.DictReader(csvfile)
            for row in reader:
                node_id = row['node_id']
                day = row['day']
                relative_fr = float(row['relative_fr']) if row['relative_fr'] else 0.0
                status = row.get('status', '')
                date_obj = parse_date(day)
                
                node_data[node_id].append({
                    'date': date_obj,
                    'day': day,
                    'relative_fr': relative_fr,
                    'status': status
                })
                all_dates.add(date_obj)
    
    except Exception as e:
        print(f"Error reading CSV for provider {provider_short}: {e}")
        return
    
    # Sort data by date for each node
    for node_id in node_data:
        node_data[node_id].sort(key=lambda x: x['date'])
    
    # Create output directory
    os.makedirs(output_dir, exist_ok=True)
    
    if not node_data:
        print(f"  No node data found for provider {provider_short}")
        return
    
    # Get sorted list of all dates
    sorted_dates = sorted(all_dates)
    
    # Create figure with two subplots for this provider
    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(16, 12))
    
    # Add reward difference percentage to the main title
    total_diff = incremental_data['total']
    thirty_day_inc = incremental_data['increment1']
    algo_inc = incremental_data['increment2']
    penalty_inc = incremental_data['increment3']
    
    diff_color = 'red' if total_diff < 0 else 'green' if total_diff > 0 else 'black'
    fig.suptitle(f'Provider {provider_short} - Node Analysis\nTotal Diff: {total_diff:+.1f}% | 30dInc: {thirty_day_inc:+.1f}% | AlgoInc: {algo_inc:+.1f}% | PenaltyInc: {penalty_inc:+.1f}%', 
                 fontsize=16, fontweight='bold', color=diff_color)
    
    # Generate bright, vibrant colors for each node
    num_nodes = len(node_data)
    
    # Create a custom bright color palette
    bright_colors = [
        '#FF0000', '#00FF00', '#0000FF', '#FFFF00', '#FF00FF', '#00FFFF',  # Red, Green, Blue, Yellow, Magenta, Cyan
        '#FF8000', '#8000FF', '#00FF80', '#FF0080', '#8000FF', '#0080FF',  # Orange, Purple, Lime, Pink, Purple, Blue
        '#FF4000', '#4000FF', '#00FF40', '#FF0040', '#4000FF', '#0040FF',  # Dark Orange, Dark Purple, Dark Lime, Dark Pink
        '#FFC000', '#C000FF', '#00FFC0', '#FF00C0', '#C000FF', '#00C0FF',  # Light Orange, Light Purple, Light Lime, Light Pink
        '#FF6000', '#6000FF', '#00FF60', '#FF0060', '#6000FF', '#0060FF',  # Medium Orange, Medium Purple, Medium Lime, Medium Pink
        '#FFA000', '#A000FF', '#00FFA0', '#FF00A0', '#A000FF', '#00A0FF',  # Light Orange, Light Purple, Light Lime, Light Pink
        '#FF2000', '#2000FF', '#00FF20', '#FF0020', '#2000FF', '#0020FF',  # Dark Orange, Dark Purple, Dark Lime, Dark Pink
        '#FFE000', '#E000FF', '#00FFE0', '#FF00E0', '#E000FF', '#00E0FF'   # Very Light Orange, Very Light Purple, Very Light Lime, Very Light Pink
    ]
    
    # If we need more colors, cycle through the palette
    if num_nodes > len(bright_colors):
        colors = bright_colors * (num_nodes // len(bright_colors) + 1)
        colors = colors[:num_nodes]
    else:
        colors = bright_colors[:num_nodes]
    
    # Plot each node's relative failure rate as a line
    for idx, (node_id, data) in enumerate(sorted(node_data.items())):
        if not data:
            continue
            
        # Prepare data for plotting
        dates = [item['date'] for item in data]
        relative_fr = [item['relative_fr'] for item in data]
        
        # Plot line for this node with bright colors and thicker lines
        color = colors[idx % len(colors)]
        ax1.plot(dates, relative_fr, marker='o', markersize=4, linewidth=2.5, 
                alpha=0.9, color=color, label=f'Node {node_id}', 
                markeredgecolor='black', markeredgewidth=0.5)
    
    # Calculate count of nodes with relative_fr > 0.1 for each day
    high_fr_counts = {}
    for date in sorted_dates:
        count = 0
        for node_id, data in node_data.items():
            # Find data for this specific date
            for item in data:
                if item['date'] == date and item['relative_fr'] > 0.1:
                    count += 1
                    break
        high_fr_counts[date] = count
    
    # Customize the first subplot (relative failure rates)
    ax1.set_title(f'Relative Failure Rates Over Time', fontsize=14)
    ax1.set_ylabel('Relative Failure Rate', fontsize=12)
    ax1.grid(True, alpha=0.3)
    
    # Format x-axis dates for first subplot
    ax1.xaxis.set_major_formatter(mdates.DateFormatter('%d-%m'))
    if len(sorted_dates) <= 15:
        ax1.xaxis.set_major_locator(mdates.DayLocator(interval=1))
    else:
        ax1.xaxis.set_major_locator(mdates.DayLocator(interval=2))
    plt.setp(ax1.xaxis.get_majorticklabels(), rotation=45)
    
    # Second subplot: Count of nodes with status "assigned" per day
    assigned_counts = {}
    high_fr_assigned_counts = {}
    unassigned_counts = {}
    
    # Read nodes_in_registry data from overall_rewards_per_day.csv
    overall_csv_path = os.path.join(provider_dir, "overall_rewards_per_day.csv")
    daily_nodes_in_registry = {}
    
    if os.path.exists(overall_csv_path):
        try:
            with open(overall_csv_path, 'r') as csvfile:
                reader = csv.DictReader(csvfile)
                for row in reader:
                    if 'day' in row and 'nodes_in_registry' in row and row['nodes_in_registry']:
                        try:
                            date_obj = parse_date(row['day'])
                            daily_nodes_in_registry[date_obj] = int(row['nodes_in_registry'])
                        except (ValueError, KeyError):
                            continue
        except Exception as e:
            print(f"  Warning: Could not read overall_rewards_per_day.csv for {provider_short}: {e}")
    
    for date in sorted_dates:
        assigned_count = 0
        high_fr_count = 0
        unassigned_count = 0
        
        # Get total nodes in registry for this date
        total_in_registry = daily_nodes_in_registry.get(date, 0)
        
        for node_id, data in node_data.items():
            # Find data for this specific date
            for item in data:
                if item['date'] == date:
                    # Count assigned nodes
                    if item['status'] == 'assigned':
                        assigned_count += 1
                        # Check if this assigned node has FR > 0.1
                        if item['relative_fr'] > 0.1:
                            high_fr_count += 1
                    break
        
        # Calculate unassigned nodes (total in registry - assigned)
        unassigned_count = max(0, total_in_registry - assigned_count)
        
        assigned_counts[date] = assigned_count
        high_fr_assigned_counts[date] = high_fr_count
        unassigned_counts[date] = unassigned_count
    
    # Calculate the remaining assigned nodes (total assigned - high FR assigned)
    remaining_assigned_counts = {}
    for date in sorted_dates:
        remaining_assigned_counts[date] = assigned_counts[date] - high_fr_assigned_counts[date]
    
    # Plot assigned nodes count as stacked bars on the second subplot
    assigned_dates = list(assigned_counts.keys())
    high_fr_values = list(high_fr_assigned_counts.values())
    remaining_values = list(remaining_assigned_counts.values())
    unassigned_values = list(unassigned_counts.values())
    
    # Create stacked bar chart: red for high FR nodes, blue for remaining assigned nodes, green for unassigned
    ax2.bar(assigned_dates, high_fr_values, alpha=0.8, 
            color='red', width=0.8, label='Assigned Nodes with FR > 0.1')
    ax2.bar(assigned_dates, remaining_values, alpha=0.7, 
            color='blue', width=0.8, bottom=high_fr_values, label='Assigned Nodes with FR ≤ 0.1')
    ax2.bar(assigned_dates, unassigned_values, alpha=0.7, 
            color='green', width=0.8, bottom=[h + r for h, r in zip(high_fr_values, remaining_values)], 
            label='Unassigned Nodes')
    
    # Customize the second subplot
    ax2.set_title(f'Total Nodes Breakdown: High FR Assigned (Red) + Normal FR Assigned (Blue) + Unassigned (Green)', fontsize=14)
    ax2.set_ylabel('Number of Nodes', fontsize=12)
    ax2.set_xlabel('Date', fontsize=12)
    ax2.grid(True, alpha=0.3)
    
    # Format x-axis dates for second subplot
    ax2.xaxis.set_major_formatter(mdates.DateFormatter('%d-%m'))
    if len(sorted_dates) <= 15:
        ax2.xaxis.set_major_locator(mdates.DayLocator(interval=1))
    else:
        ax2.xaxis.set_major_locator(mdates.DayLocator(interval=2))
    plt.setp(ax2.xaxis.get_majorticklabels(), rotation=45)
    
    # Set y-axis limits for total nodes count (assigned + unassigned)
    total_nodes_per_day = [a + u for a, u in zip(assigned_counts.values(), unassigned_counts.values())]
    max_total = max(total_nodes_per_day) if total_nodes_per_day else 0
    ax2.set_ylim(0, max_total + 1)
    
    # Add legend to first subplot (but limit it if too many nodes)
    if num_nodes <= 15:
        ax1.legend(bbox_to_anchor=(1.05, 1), loc='upper left', fontsize=8)
    else:
        # For providers with many nodes, show a simplified legend
        ax1.text(1.02, 0.5, f'{num_nodes} nodes\n(colors vary)', transform=ax1.transAxes, 
                fontsize=10, verticalalignment='center',
                bbox=dict(boxstyle="round,pad=0.3", facecolor="lightgray"))
    
    # Add legend to second subplot
    ax2.legend(loc='upper right', fontsize=10)
    
    # Add reward difference info box on the second subplot
    # Get the actual reward values for display
    json_reward = incremental_data['json_reward']
    json_30day = incremental_data['json_30day']
    no_penalties_reward = incremental_data['no_penalties_reward']
    with_penalties_reward = incremental_data['with_penalties_reward']
    
    diff_text = f'Total Difference: {total_diff:+.1f}%\n'
    diff_text += f'30dInc (30-day): {thirty_day_inc:+.1f}%\n'
    diff_text += f'AlgoInc (New Algo): {algo_inc:+.1f}%\n'
    diff_text += f'PenaltyInc (Penalties): {penalty_inc:+.1f}%\n'
    diff_text += f'Last Rewards 30d: {json_30day:,.0f}\n'
    diff_text += f'No Penalties: {no_penalties_reward:,.0f}\n'
    diff_text += f'With Penalties: {with_penalties_reward:,.0f}'
    
    ax2.text(0.02, 0.98, diff_text, transform=ax2.transAxes, 
             fontsize=11, verticalalignment='top', color=diff_color,
             bbox=dict(boxstyle="round,pad=0.5", facecolor="lightgray", alpha=0.8))
    
    # Adjust layout and save
    plt.tight_layout(rect=[0, 0.03, 1, 0.95])
    
    # Save the plot
    plot_filename = f"provider_{provider_short}_relative_failure_rates.png"
    plot_path = os.path.join(output_dir, plot_filename)
    plt.savefig(plot_path, dpi=200, bbox_inches='tight')
    plt.close()
    
    print(f"  Created single plot for provider {provider_short} with {num_nodes} nodes (Total Diff: {total_diff:+.1f}%, 30dInc: {thirty_day_inc:+.1f}%, AlgoInc: {algo_inc:+.1f}%, PenaltyInc: {penalty_inc:+.1f}%)")

def main():
    """Main function to process all providers and create plots."""
    result_dir = "/Users/pietro.di.marco/Documents/dfinity/082025_results/082025_results_with_penalties/rewards_results/result"
    no_penalties_dir = "/Users/pietro.di.marco/Documents/dfinity/082025_results/082025_results_no_penalties/rewards_results/result"
    output_dir = "relative_failure_rate_plots"
    
    # Create output directory
    os.makedirs(output_dir, exist_ok=True)
    
    if not os.path.exists(result_dir):
        print(f"Error: {result_dir} directory not found")
        return
    
    if not os.path.exists(no_penalties_dir):
        print(f"Error: {no_penalties_dir} directory not found")
        return
    
    print("Reading CSV data...")
    csv_rewards, csv_nodes_assigned, csv_nodes_in_registry = read_csv_rewards_and_nodes(result_dir)
    
    print("Reading no penalties CSV data...")
    no_penalties_rewards, _, _ = read_csv_rewards_and_nodes(no_penalties_dir)
    
    print("Loading JSON rewards data...")
    script_dir = Path(__file__).parent
    json_path = script_dir / "last_rewards_received.json"
    
    if not os.path.exists(json_path):
        print(f"Error: {json_path} not found")
        return
    
    reward_data = load_last_rewards_received(json_path)
    json_rewards = compute_xdr_rewards_from_json(reward_data)
    
    # Get all provider directories
    provider_dirs = [d for d in os.listdir(result_dir) 
                    if os.path.isdir(os.path.join(result_dir, d)) 
                    and d != '__pycache__']
    
    print(f"\nProcessing {len(provider_dirs)} providers")
    print("Creating single plot per provider showing relative failure rates over time")
    print("=" * 70)
    
    # Process each provider
    for provider_dir_name in sorted(provider_dirs):
        provider_path = os.path.join(result_dir, provider_dir_name)
        
        try:
            # Create single plot showing relative failure rates for all nodes
            create_provider_node_plots(provider_path, output_dir, json_rewards, no_penalties_rewards, csv_rewards)
            
        except Exception as e:
            print(f"Error processing provider {truncate_provider_id(provider_dir_name)}: {e}")
            continue
    
    print("=" * 70)
    print(f"Plot generation complete! Check the '{output_dir}' directory for results.")
    
    # Print summary of what was created
    plot_files = [f for f in os.listdir(output_dir) if f.endswith('.png')]
    print(f"\nTotal relative failure rate plot files created: {len(plot_files)}")
    
    print("\nSample of created files:")
    for plot_file in sorted(plot_files)[:10]:
        print(f"  {plot_file}")
    
    if len(plot_files) > 10:
        print(f"  ... and {len(plot_files) - 10} more files")
    
    print(f"\nAll providers processed:")
    for provider in sorted(provider_dirs):
        incremental_data = calculate_three_incremental_differences(provider, json_rewards, no_penalties_rewards, csv_rewards)
        print(f"  {truncate_provider_id(provider)}: Total {incremental_data['total']:+.1f}%, 30dInc {incremental_data['increment1']:+.1f}%, AlgoInc {incremental_data['increment2']:+.1f}%, PenaltyInc {incremental_data['increment3']:+.1f}%")

if __name__ == "__main__":
    main()
