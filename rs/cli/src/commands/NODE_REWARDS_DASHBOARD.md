# Node Provider Rewards Dashboard

## Overview

This comprehensive Grafana dashboard visualizes node provider rewards metrics from the IC Network. It provides insights into provider performance, node failures, subnet health, and rewards distribution.

## Dashboard File

- **Main Dashboard**: `node_rewards_dashboard.json`
- **Original Example**: `dashboard.json`

## Required Metrics

The dashboard uses the following Prometheus metrics exported by the node rewards canister:

### Provider-Level Metrics
- `latest_nodes_count{provider_id}` - Number of nodes per provider
- `latest_total_base_rewards_xdr_permyriad{provider_id}` - Base rewards without performance adjustments
- `latest_total_adjusted_rewards_xdr_permyriad{provider_id}` - Performance-adjusted rewards
- `latest_rewards_calculation_date{date}` - Timestamp of latest calculation

### Node-Level Metrics
- `latest_original_failure_rate{node_id, provider_id, subnet_id}` - Raw failure rate per node
- `latest_relative_failure_rate{node_id, provider_id, subnet_id}` - Relative failure rate compared to subnet average

### Subnet-Level Metrics
- `subnet_failure_rate{subnet_id}` - Average failure rate per subnet

## Dashboard Sections

### 1. Provider Overview
**Purpose**: High-level summary of all providers and total network statistics

**Panels**:
- **Latest Rewards Calculation**: Shows when rewards were last calculated
- **Total Providers**: Count of all tracked providers
- **Total Nodes**: Sum of all nodes across providers
- **Total Adjusted Rewards**: Network-wide rewards (XDR Permyriad)
- **Provider Rewards Summary Table**: Comprehensive table with:
  - Total nodes per provider
  - Base rewards total
  - Adjusted rewards total
  - Difference (Adjusted - Base)
  - Efficiency percentage with color-coded gauge

**Use Case**: Quick overview of network health and provider distribution

### 2. Provider Rewards Trends
**Purpose**: Time-series analysis of rewards and efficiency

**Panels**:
- **Base vs Adjusted Rewards Over Time**: Line chart comparing base and adjusted rewards
  - Blue lines = Base rewards
  - Green lines = Adjusted rewards
  - Shows trend of performance impact on rewards
  
- **Provider Rewards Efficiency Over Time**: Line chart showing efficiency percentage
  - Green threshold: 98%+ (excellent performance)
  - Yellow threshold: 95-98% (good performance)
  - Red threshold: <90% (underperforming)
  
- **Node Counts Over Time**: Tracks changes in node count per provider
  
- **Provider Rewards Penalty**: Shows rewards lost due to poor performance
  - Calculated as: Base - Adjusted
  - Higher values indicate more performance issues

**Use Case**: Monitor provider performance trends, identify degradation early

### 3. Node-Level Metrics
**Purpose**: Detailed analysis of individual node performance

**Panels**:
- **Node Original Failure Rate**: Raw failure rate per node
  - Green: <5% (healthy)
  - Yellow: 5-10% (warning)
  - Red: >10% (critical)
  
- **Node Relative Failure Rate**: Failure rate relative to subnet average
  - Shows if node is performing worse than peers
  
- **Original Failure Rate Heatmap**: Visual distribution of failures over time
  - Darker colors indicate higher failure rates
  - Helps identify patterns and anomalies
  
- **Relative Failure Rate Heatmap**: Comparative performance heatmap
  
- **Node Performance Details Table**: Sortable table with all node metrics
  - Includes both original and relative failure rates
  - Color-coded gauges for quick identification
  - Pagination for large datasets
  - Clickable subnet IDs for drilling down

**Use Case**: Identify underperforming nodes, troubleshoot specific issues

### 4. Subnet Metrics
**Purpose**: Subnet-level health monitoring

**Panels**:
- **Subnet Failure Rates Over Time**: Line chart showing all subnet failure rates
  - Helps identify problematic subnets
  - Compare relative subnet health
  
- **Subnet Failure Rates Comparison Table**: Sortable table
  - Ranked by failure rate
  - Color-coded gauges
  
- **Node Distribution Across Subnets**: Pie chart
  - Shows how nodes are distributed
  - Identifies subnet load balance
  
- **Subnet Failure Rates Bar Chart**: Horizontal bar chart
  - Sorted by failure rate (worst first)
  - Quick visual comparison

**Use Case**: Monitor subnet health, identify capacity issues

### 5. Analysis & Insights
**Purpose**: High-level analytics and rankings

**Panels**:
- **Top 10 Providers by Adjusted Rewards**: Bar chart
  - Identifies largest reward recipients
  
- **Top 10 Providers by Node Count**: Bar chart
  - Shows provider scale distribution
  
- **Bottom 10 Providers by Efficiency**: Bar chart
  - Highlights providers needing attention
  - Color-coded by efficiency thresholds
  
- **Provider Efficiency Distribution**: Histogram
  - Shows how many providers fall into each efficiency bucket:
    - 98-100%: Excellent
    - 95-98%: Good
    - 90-95%: Fair
    - <90%: Poor

**Use Case**: Identify trends, outliers, and areas needing intervention

## Dashboard Variables

The dashboard includes interactive filters:

### Provider Filter (`$provider`)
- **Type**: Multi-select dropdown
- **Options**: All tracked provider IDs
- **Default**: All
- **Use**: Filter all panels to specific provider(s)

### Node Filter (`$node`)
- **Type**: Multi-select dropdown
- **Options**: All nodes from selected provider(s)
- **Default**: All
- **Use**: Filter node-level panels to specific node(s)

### Subnet Filter (`$subnet`)
- **Type**: Multi-select dropdown
- **Options**: All subnet IDs
- **Default**: All
- **Use**: Filter subnet-related panels

## Importing the Dashboard

### Method 1: Grafana UI
1. Open Grafana
2. Navigate to **Dashboards** → **Import**
3. Click **Upload JSON file**
4. Select `node_rewards_dashboard.json`
5. Configure data source (select your Prometheus instance)
6. Click **Import**

### Method 2: Grafana API
```bash
curl -X POST \
  http://your-grafana-url/api/dashboards/db \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -d @node_rewards_dashboard.json
```

### Method 3: Provisioning
1. Copy `node_rewards_dashboard.json` to Grafana provisioning directory:
   ```bash
   cp node_rewards_dashboard.json /etc/grafana/provisioning/dashboards/
   ```
2. Restart Grafana

## Configuration

### Data Source
The dashboard uses a variable `${DS_PROMETHEUS}` for the Prometheus data source. 

To configure:
1. Edit the dashboard
2. Click **Settings** (gear icon)
3. Select **Variables**
4. Edit `DS_PROMETHEUS`
5. Select your Prometheus instance

### Time Range
- **Default**: Last 30 days
- **Recommended**: Adjust based on your data retention policy
- Supports real-time updates with auto-refresh

### Refresh Rate
- **Default**: 1 minute
- **Options**: 10s, 30s, 1m, 5m, 15m, 30m, 1h, 2h, 1d
- Configurable via dropdown in top-right corner

## Interpreting Metrics

### XDR Permyriad
- Unit used for rewards calculation
- 1 XDR = 10,000 XDR Permyriad
- Example: 13,872,068 XDR Permyriad = 1,387.2 XDR

### Efficiency Percentage
```
Efficiency % = (Adjusted Rewards / Base Rewards) × 100
```
- **100%**: Perfect performance, no penalties
- **98-100%**: Excellent, minor issues
- **95-98%**: Good, some performance impact
- **90-95%**: Fair, noticeable performance issues
- **<90%**: Poor, significant performance problems

### Failure Rates
- **Original Failure Rate**: Raw percentage of failed blocks
- **Relative Failure Rate**: Comparison to subnet average
  - Positive = worse than average
  - Negative = better than average
  - Zero = at subnet average

## Alerting

### Recommended Alerts

1. **High Provider Penalty**
   ```promql
   (latest_total_base_rewards_xdr_permyriad - latest_total_adjusted_rewards_xdr_permyriad) > 500000
   ```
   Triggers when a provider loses >50 XDR in rewards

2. **Low Efficiency**
   ```promql
   (latest_total_adjusted_rewards_xdr_permyriad / latest_total_base_rewards_xdr_permyriad) * 100 < 95
   ```
   Triggers when efficiency drops below 95%

3. **High Node Failure Rate**
   ```promql
   latest_original_failure_rate > 0.1
   ```
   Triggers when any node exceeds 10% failure rate

4. **High Subnet Failure Rate**
   ```promql
   subnet_failure_rate > 0.1
   ```
   Triggers when subnet failure rate exceeds 10%

## Troubleshooting

### No Data Displayed
1. Verify Prometheus data source is configured correctly
2. Check that metrics are being scraped from node rewards canister
3. Verify time range includes data points
4. Check Prometheus query inspector for errors

### Slow Performance
1. Reduce time range
2. Apply provider/node filters
3. Increase refresh interval
4. Check Prometheus query performance

### Missing Panels
1. Verify all required metrics are available in Prometheus
2. Check metric naming conventions match exactly
3. Review Grafana logs for errors

## Command Output Mapping

The dashboard visualizes data that corresponds to the CLI command output:

### From `DailyRewardSummary`
- Base Rewards Total → `latest_total_base_rewards_xdr_permyriad`
- Adjusted Rewards Total → `latest_total_adjusted_rewards_xdr_permyriad`
- Adjusted Rewards % → Calculated in panels
- Nodes → `latest_nodes_count`

### From Node Metrics CSVs
- `node_metrics_by_day.csv` → Time-series panels
- `node_metrics_by_node.csv` → Node Performance Details table
- `node_metrics_by_performance_multiplier.csv` → Efficiency analysis

### From Subnet Data
- `subnets_failure_rates.csv` → Subnet Metrics section panels

## Extending the Dashboard

### Adding New Panels
1. Click **Add** → **Visualization**
2. Select data source: `${DS_PROMETHEUS}`
3. Write PromQL query using available metrics
4. Configure visualization type and options
5. Save dashboard

### Creating Custom Variables
1. Dashboard Settings → Variables
2. Add new variable
3. Query: `label_values(metric_name, label_name)`
4. Use in panels: `{label=~"$variable_name"}`

### Example Custom Queries

**Average efficiency per provider:**
```promql
avg_over_time((latest_total_adjusted_rewards_xdr_permyriad{provider_id=~"$provider"} / 
latest_total_base_rewards_xdr_permyriad{provider_id=~"$provider"} * 100)[7d:1d])
```

**Node count change rate:**
```promql
rate(latest_nodes_count{provider_id=~"$provider"}[1h]) * 3600
```

**Subnet with highest average failure rate:**
```promql
topk(1, avg_over_time(subnet_failure_rate[7d]))
```

## Data Sources

### Node Rewards Canister
The metrics are exported from the Node Rewards Canister which:
- Fetches daily node performance data
- Calculates rewards based on performance multipliers
- Exports metrics in Prometheus format
- Updates hourly via scheduled tasks

### Related Commands
```bash
# View ongoing rewards
dre node-rewards ongoing

# View past month rewards with comparison
dre node-rewards past-rewards 2025-11 --compare-with-governance

# Export detailed CSVs
dre node-rewards ongoing --csv-detailed-output-path ./output

# Filter to specific provider
dre node-rewards ongoing --provider-id provider_prefix
```

## Dashboard Maintenance

### Regular Updates
- Dashboard structure: As needed when new metrics added
- Variables: Auto-updated from Prometheus
- Data retention: Follow Prometheus retention policy

### Version Control
Keep `node_rewards_dashboard.json` in version control:
```bash
git add rs/cli/src/commands/node_rewards_dashboard.json
git commit -m "Update node rewards dashboard"
```

### Backup
Export dashboard JSON regularly:
1. Dashboard Settings → JSON Model
2. Copy JSON
3. Save to file

## Support

For issues or questions:
1. Check Grafana documentation: https://grafana.com/docs/
2. Verify Prometheus metrics: `http://prometheus:9090/metrics`
3. Review Node Rewards Canister logs
4. Consult the DRE team

## Related Documentation

- [Node Rewards Command](./node_rewards/mod.rs) - CLI implementation
- [Release Notes](../../../../../docs/make-release.md) - Release process
- [Node Rewards Docs](../../../../../docs/node-rewards.md) - Detailed explanation

## Changelog

### Version 1.0.0 (Initial)
- Provider overview section with summary stats
- Provider rewards trends over time
- Node-level failure rate analysis
- Subnet health monitoring
- Analysis & insights section
- Interactive filters for provider, node, and subnet
- Comprehensive tables with sorting and color coding
- Heatmaps for failure rate distribution
- Top/bottom provider rankings

