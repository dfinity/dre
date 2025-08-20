#!/usr/bin/env python3
"""
Create line plots for each node provider showing relative failure rates over time.
One single plot per provider with all nodes displayed as different colored lines.
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

def truncate_provider_id(provider_id):
    """Truncate provider ID to just the part before the first dash."""
    return provider_id.split('-')[0]

def parse_date(date_str):
    """Parse date string in DD-MM-YYYY format to datetime object."""
    return datetime.strptime(date_str, "%d-%m-%Y")

def create_provider_node_plots(provider_dir, output_dir):
    """Create a single plot per provider showing relative failure rates for all nodes over time."""
    provider_name = os.path.basename(provider_dir)
    provider_short = truncate_provider_id(provider_name)
    
    nodes_csv_path = os.path.join(provider_dir, "nodes_results.csv")
    
    if not os.path.exists(nodes_csv_path):
        print(f"Warning: nodes_results.csv not found for provider {provider_short}")
        return
    
    print(f"Processing provider: {provider_short}")
    
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
    fig.suptitle(f'Provider {provider_short} - Node Analysis', fontsize=16, fontweight='bold')
    
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
    
    # Create a second y-axis for the high failure rate count on the first subplot
    ax1_twin = ax1.twinx()
    
    # Plot high failure rate count as bars on the secondary y-axis
    high_fr_dates = list(high_fr_counts.keys())
    high_fr_values = list(high_fr_counts.values())
    
    # Use a different color for the bars (dark red with transparency)
    bars = ax1_twin.bar(high_fr_dates, high_fr_values, alpha=0.3, 
                        color='darkred', width=0.8, label='Nodes with FR > 0.1 Count')
    
    # Customize the secondary y-axis
    ax1_twin.set_ylabel('Count of Nodes with FR > 0.1', fontsize=12, color='darkred')
    ax1_twin.tick_params(axis='y', labelcolor='darkred')
    
    # Set y-axis limits for high failure rate count
    max_count = max(high_fr_values) if high_fr_values else 0
    ax1_twin.set_ylim(0, max_count + 1)
    
    # Customize the first subplot (relative failure rates)
    ax1.set_title(f'Relative Failure Rates Over Time + Count of Nodes with FR > 0.1', fontsize=14)
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
    for date in sorted_dates:
        count = 0
        for node_id, data in node_data.items():
            # Find data for this specific date
            for item in data:
                if item['date'] == date and item['status'] == 'assigned':
                    count += 1
                    break
        assigned_counts[date] = count
    
    # Plot assigned nodes count as bars on the second subplot
    assigned_dates = list(assigned_counts.keys())
    assigned_values = list(assigned_counts.values())
    
    # Use a different color for the assigned nodes bars (dark blue with transparency)
    ax2.bar(assigned_dates, assigned_values, alpha=0.7, 
            color='darkblue', width=0.8, label='Assigned Nodes Count')
    
    # Customize the second subplot
    ax2.set_title(f'Count of Nodes with Status "Assigned" Per Day', fontsize=14)
    ax2.set_ylabel('Number of Assigned Nodes', fontsize=12)
    ax2.set_xlabel('Date', fontsize=12)
    ax2.grid(True, alpha=0.3)
    
    # Format x-axis dates for second subplot
    ax2.xaxis.set_major_formatter(mdates.DateFormatter('%d-%m'))
    if len(sorted_dates) <= 15:
        ax2.xaxis.set_major_locator(mdates.DayLocator(interval=1))
    else:
        ax2.xaxis.set_major_locator(mdates.DayLocator(interval=2))
    plt.setp(ax2.xaxis.get_majorticklabels(), rotation=45)
    
    # Set y-axis limits for assigned nodes count
    max_assigned = max(assigned_values) if assigned_values else 0
    ax2.set_ylim(0, max_assigned + 1)
    
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
    
    # Adjust layout and save
    plt.tight_layout(rect=[0, 0.03, 1, 0.95])
    
    # Save the plot
    plot_filename = f"provider_{provider_short}_relative_failure_rates.png"
    plot_path = os.path.join(output_dir, plot_filename)
    plt.savefig(plot_path, dpi=200, bbox_inches='tight')
    plt.close()
    
    print(f"  Created single plot for provider {provider_short} with {num_nodes} nodes")

def main():
    """Main function to process all providers and create plots."""
    result_dir = "result"
    output_dir = "relative_failure_rate_plots"
    
    # Create output directory
    os.makedirs(output_dir, exist_ok=True)
    
    if not os.path.exists(result_dir):
        print(f"Error: {result_dir} directory not found")
        return
    
    # Get all provider directories
    provider_dirs = [d for d in os.listdir(result_dir) 
                    if os.path.isdir(os.path.join(result_dir, d)) 
                    and d != '__pycache__']
    
    print(f"Found {len(provider_dirs)} providers to process")
    print("Creating single plot per provider showing relative failure rates over time")
    print("=" * 70)
    
    # Process each provider
    for provider_dir_name in sorted(provider_dirs):
        provider_path = os.path.join(result_dir, provider_dir_name)
        
        try:
            # Create single plot showing relative failure rates for all nodes
            create_provider_node_plots(provider_path, output_dir)
            
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

if __name__ == "__main__":
    main()
