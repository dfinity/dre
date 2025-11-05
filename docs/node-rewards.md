# Node Rewards Command

The `node-rewards` command allows node providers to check and analyze their rewards by querying the Node Rewards Canister (NRC) and comparing them with governance rewards from the NNS. This tool provides comprehensive insights into daily rewards, performance metrics, and helps identify discrepancies between calculated rewards and actual governance payouts.

## Overview

The `node-rewards` command fetches reward data from two sources:

1. **Node Rewards Canister (NRC)**: Calculates rewards based on daily node performance metrics
2. **Governance (NNS)**: Actual rewards paid out through governance snapshots

By comparing these two sources, node providers can:
- Verify that their rewards match expectations
- Identify any discrepancies between calculated and actual rewards
- Monitor daily performance and identify underperforming nodes
- Export detailed CSV reports for further analysis

## Command Structure

The command has two modes:

```bash
dre node-rewards <mode> [options]
```

### Modes

1. **`ongoing`**: Shows rewards from the latest governance snapshot timestamp to yesterday
2. **`past-rewards <month>`**: Shows past rewards for a specific month (format: `YYYY-MM`) and compares with governance

### Options

Both modes support the following options:

- `--csv-detailed-output-path <path>`: If set, writes detailed CSV files to the specified directory
- `--provider-id <id>`: Filter to a single provider (full principal ID or provider prefix)

## Usage Examples

### Check Ongoing Rewards

View rewards from the latest governance snapshot to yesterday:

```bash
dre node-rewards ongoing
```

This will display a daily rewards summary table showing:
- Daily base and adjusted rewards totals
- Number of nodes and assigned nodes
- Underperforming nodes (nodes with performance multiplier < 1.0)
- Adjusted rewards percentage

### Check Past Rewards for a Specific Month

View and compare rewards for a past month (e.g., October 2024):

```bash
dre node-rewards past-rewards 2024-10
```

This mode provides:
- Daily rewards summary for the specified month
- Comparison table showing NRC vs Governance rewards
- Difference and percentage difference calculations
- List of underperforming nodes

### Filter by Provider ID

Check rewards for a specific provider using their full principal ID or prefix:

```bash
dre node-rewards ongoing --provider-id tm3pc-2bjsx-hhv3v-fsrt7-wotdj-nbu3t-ewloq-uporp-tacou-lupdn-oae
```

Or using just the prefix:

```bash
dre node-rewards ongoing --provider-id tm3pc
```

### Export Detailed CSV Reports

Generate comprehensive CSV files for further analysis:

```bash
dre node-rewards ongoing --csv-detailed-output-path ./rewards_export
```

This creates a directory structure like:

```
rewards_export/
└── rewards_2024-10-01_to_2024-10-31/
    ├── <provider_id>/
    │   ├── base_rewards.csv
    │   ├── base_rewards_type3.csv
    │   ├── rewards_summary.csv
    │   ├── node_metrics_by_day.csv
    │   └── node_metrics_by_node.csv
    └── subnets_failure_rates.csv
```

### Complete Example: Export Past Month with Provider Filter

```bash
dre node-rewards past-rewards 2024-10 \
    --provider-id tm3pc \
    --csv-detailed-output-path ./october_rewards
```

## Understanding the Output

### Daily Rewards Summary

When viewing rewards in console mode (without CSV export), you'll see a table for each provider showing:

| Column | Description |
|--------|-------------|
| **Day** | UTC day (YYYY-MM-DD) |
| **Base Rewards Total** | Sum of `base_rewards_xdr_permyriad` across all nodes (XDRPermyriad) |
| **Adjusted Rewards Total** | Sum of `adjusted_rewards_xdr_permyriad` across all nodes (XDRPermyriad) |
| **Adjusted Rewards %** | (Adjusted Rewards Total / Base Rewards Total) × 100% |
| **Nodes** | Total nodes found in registry on that day |
| **Assigned** | Number of nodes assigned to a subnet on that day |
| **Underperf** | Number of nodes with performance multiplier < 1.0 |
| **Underperf Nodes** | Comma-separated list of underperforming node IDs (prefixes) |

### Comparison Table (Past Rewards Mode)

When using `past-rewards` mode, an additional comparison table is displayed:

| Column | Description |
|--------|-------------|
| **Provider** | Provider ID prefix |
| **NRC** | Total rewards from Node Rewards Canister (XDRPermyriad) |
| **Governance** | Total rewards from NNS governance (XDRPermyriad) |
| **Difference** | NRC - Governance (XDRPermyriad) |
| **% Diff** | (Difference / NRC) × 100% |
| **Underperforming Nodes** | List of node prefixes with performance issues |

!!! tip "Understanding Rewards Units"
    All rewards are displayed in **XDRPermyriad** (XDR per ten-thousand). This is a standardized unit used for reward calculations on the IC.

!!! note "Performance Multiplier"
    Nodes with a performance multiplier < 1.0 are considered underperforming. This multiplier affects the adjusted rewards based on the node's actual performance metrics.

## CSV Export Files

When using `--csv-detailed-output-path`, the following CSV files are generated:

### Per-Provider Files

Each provider gets a directory with these files:

#### `base_rewards.csv`
Contains base reward calculations by reward type and region:
- `day_utc`: Date in UTC
- `monthly_xdr_permyriad`: Monthly base reward
- `daily_xdr_permyriad`: Daily base reward
- `node_reward_type`: Type of reward (e.g., Type1, Type2, Type3)
- `region`: Geographic region

#### `base_rewards_type3.csv`
Contains Type3 reward calculations (region-based rewards):
- `day_utc`: Date in UTC
- `value_xdr_permyriad`: Daily reward value
- `region`: Geographic region
- `nodes_count`: Number of nodes in this region
- `avg_rewards_xdr_permyriad`: Average reward per node
- `avg_coefficient`: Average coefficient applied

#### `rewards_summary.csv`
Daily summary of rewards per provider:
- `day_utc`: Date in UTC
- `base_rewards_total`: Total base rewards for the day
- `adjusted_rewards_total`: Total adjusted rewards after performance multipliers
- `adjusted_rewards_percent`: Percentage of base rewards received
- `rewards_total_xdr_permyriad`: Total rewards for the day
- `nodes_in_registry`: Total nodes in registry
- `assigned_nodes`: Number of nodes assigned to subnets
- `underperforming_nodes_count`: Count of underperforming nodes
- `underperforming_nodes`: Comma-separated list of node IDs

#### `node_metrics_by_day.csv`
Node-level metrics organized by day:
- `day_utc`: Date in UTC
- `node_id`: Full node principal ID
- `node_reward_type`: Reward type for the node
- `region`: Geographic region
- `dc_id`: Datacenter ID
- `node_status`: Assigned, Unassigned, or Unknown
- `performance_multiplier`: Performance multiplier (1.0 = full performance)
- `rewards_reduction`: Reduction applied to rewards
- `base_rewards_xdr_permyriad`: Base rewards for this node
- `adjusted_rewards_xdr_permyriad`: Adjusted rewards after multiplier
- `subnet_assigned`: Subnet ID if assigned
- `subnet_assigned_failure_rate`: Failure rate of the subnet
- `num_blocks_proposed`: Number of blocks proposed
- `num_blocks_failed`: Number of blocks that failed
- `original_failure_rate`: Original failure rate
- `relative_failure_rate`: Relative failure rate
- `extrapolated_failure_rate`: Extrapolated failure rate (for unassigned nodes)

#### `node_metrics_by_node.csv`
Same data as `node_metrics_by_day.csv`, but organized by node ID (all days for a node are grouped together).

### Global Files

#### `subnets_failure_rates.csv`
Subnet-level failure rate data:
- `subnet_id`: Subnet principal ID
- `day_utc`: Date in UTC
- `failure_rate`: Failure rate for the subnet on that day

## Common Use Cases

### 1. Monthly Reward Verification

At the end of each month, verify that your rewards match expectations:

```bash
dre node-rewards past-rewards 2024-10 --provider-id <your-provider-id>
```

Review the comparison table to ensure NRC and Governance rewards match. If there's a significant difference, investigate the causes (e.g., underperforming nodes, timing differences).

### 2. Daily Performance Monitoring

Check ongoing rewards to monitor daily performance:

```bash
dre node-rewards ongoing --provider-id <your-provider-id>
```

Look for:
- Sudden drops in adjusted rewards percentage
- New underperforming nodes
- Changes in assigned node count

### 3. Export for Analysis

Generate CSV files for detailed analysis in spreadsheet tools:

```bash
dre node-rewards past-rewards 2024-10 \
    --provider-id <your-provider-id> \
    --csv-detailed-output-path ./reports
```

Then analyze:
- Trends in `node_metrics_by_node.csv` to identify problematic nodes
- Performance patterns in `rewards_summary.csv`
- Subnet failure rates in `subnets_failure_rates.csv`

### 4. Identifying Underperforming Nodes

The command automatically identifies nodes with performance multiplier < 1.0. Review the "Underperf Nodes" column to see which nodes need attention:

```bash
dre node-rewards ongoing --provider-id <your-provider-id>
```

Common causes for underperformance:
- High failure rates
- Network issues
- Subnet-specific problems
