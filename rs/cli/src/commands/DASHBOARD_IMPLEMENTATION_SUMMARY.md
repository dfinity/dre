# Node Rewards Dashboard Implementation Summary

## Overview

This document summarizes the comprehensive Grafana dashboard implementation for visualizing node provider rewards data from the IC Network.

## Files Created

1. **`node_rewards_dashboard.json`** (Main Dashboard)
   - Complete Grafana dashboard JSON
   - 25+ panels organized into 5 sections
   - Interactive filters and variables
   - ~1800 lines of JSON configuration

2. **`NODE_REWARDS_DASHBOARD.md`** (Documentation)
   - Complete usage guide
   - Panel descriptions
   - Configuration instructions
   - Troubleshooting guide
   - Example queries

3. **`DASHBOARD_IMPLEMENTATION_SUMMARY.md`** (This file)
   - Implementation overview
   - Mapping between command output and dashboard

## Dashboard Sections Breakdown

### Section 1: Provider Overview (Row ID: 100)
**Panels: 5**

| Panel ID | Panel Name | Metric(s) | Command Output Mapping |
|----------|-----------|-----------|------------------------|
| 101 | Latest Rewards Calculation | `latest_rewards_calculation_date` | N/A - System metadata |
| 102 | Total Providers | `count(latest_nodes_count)` | Count of providers in output |
| 103 | Total Nodes | `sum(latest_nodes_count)` | Sum of "Nodes" column |
| 104 | Total Adjusted Rewards | `sum(latest_total_adjusted_rewards_xdr_permyriad)` | Sum of "Adjusted Total" |
| 105 | Provider Rewards Summary | Multiple metrics combined | Comprehensive table from `print_comparison_console` |

**Maps to Command Output:**
- `print_comparison_console()` comparison table
- Provider-level aggregations

### Section 2: Provider Rewards Trends (Row ID: 200)
**Panels: 4**

| Panel ID | Panel Name | Metric(s) | Command Output Mapping |
|----------|-----------|-----------|------------------------|
| 201 | Base vs Adjusted Rewards Over Time | `latest_total_base_rewards_xdr_permyriad`, `latest_total_adjusted_rewards_xdr_permyriad` | Time series of "Base Rewards Total" and "Adjusted Rewards Total" from `print_daily_summary_console` |
| 202 | Provider Rewards Efficiency Over Time | Calculated: `(adjusted/base)*100` | "Adjusted Rewards %" column over time |
| 203 | Node Counts Over Time | `latest_nodes_count` | "Nodes" column time series |
| 204 | Provider Rewards Penalty | Calculated: `base - adjusted` | "Adj-Base Diff" column |

**Maps to Command Output:**
- `DailyRewardSummary` struct fields
- `print_daily_summary_console()` output
- `rewards_summary.csv` when using `--csv-detailed-output-path`

### Section 3: Node-Level Metrics (Row ID: 300)
**Panels: 5**

| Panel ID | Panel Name | Metric(s) | Command Output Mapping |
|----------|-----------|-----------|------------------------|
| 301 | Node Original Failure Rate | `latest_original_failure_rate` | `original_failure_rate` from `node_metrics_by_*.csv` |
| 302 | Node Relative Failure Rate | `latest_relative_failure_rate` | `relative_failure_rate` from `node_metrics_by_*.csv` |
| 303 | Original Failure Rate Heatmap | `latest_original_failure_rate` (heatmap) | Visual aggregation of failure rates |
| 304 | Relative Failure Rate Heatmap | `latest_relative_failure_rate` (heatmap) | Visual aggregation of relative rates |
| 305 | Node Performance Details | Combined node metrics | `node_metrics_by_node.csv` content |

**Maps to Command Output:**
- `create_node_metrics_csv()` outputs:
  - `node_metrics_by_day.csv`
  - `node_metrics_by_node.csv`
  - `node_metrics_by_performance_multiplier.csv`
- `DailyNodeFailureRate` enum variants
- `NodeMetricsDaily` struct

### Section 4: Subnet Metrics (Row ID: 400)
**Panels: 4**

| Panel ID | Panel Name | Metric(s) | Command Output Mapping |
|----------|-----------|-----------|------------------------|
| 401 | Subnet Failure Rates Over Time | `subnet_failure_rate` | Time series from `subnets_failure_rates.csv` |
| 402 | Subnet Failure Rates Comparison | `subnet_failure_rate` (table) | Table view of `subnets_failure_rates.csv` |
| 403 | Node Distribution Across Subnets | `count by (subnet_id)` | Distribution of `subnet_assigned` field |
| 404 | Subnet Failure Rates Bar Chart | `subnet_failure_rate` (sorted) | Sorted bar chart of subnet rates |

**Maps to Command Output:**
- `create_subnets_failure_rates_csv()` output
- `NrcData.subnets_failure_rates` field
- `subnet_assigned_failure_rate` from node metrics

### Section 5: Analysis & Insights (Row ID: 500)
**Panels: 4**

| Panel ID | Panel Name | Metric(s) | Command Output Mapping |
|----------|-----------|-----------|------------------------|
| 501 | Top 10 Providers by Adjusted Rewards | `topk(10, latest_total_adjusted_rewards_xdr_permyriad)` | Top 10 from "Adjusted" column in comparison table |
| 502 | Top 10 Providers by Node Count | `topk(10, latest_nodes_count)` | Top 10 from "Nodes" column |
| 503 | Bottom 10 Providers by Efficiency | `bottomk(10, efficiency_calc)` | Bottom 10 from "Adj-Base %" or "Adj-Gov %" |
| 504 | Provider Efficiency Distribution | Count by efficiency buckets | Statistical distribution of efficiency |

**Maps to Command Output:**
- Aggregations and rankings from `print_comparison_console()`
- Statistical analysis not directly shown in CLI but derivable from data

## Data Flow

```
┌─────────────────────────────────────┐
│  Node Rewards Canister              │
│  - Fetches daily node data          │
│  - Calculates rewards               │
│  - Computes performance multipliers │
└──────────────┬──────────────────────┘
               │ exports metrics
               ▼
┌─────────────────────────────────────┐
│  Prometheus                         │
│  - Scrapes metrics endpoint         │
│  - Stores time series               │
│  - Provides PromQL query interface  │
└──────────────┬──────────────────────┘
               │ queries
               ▼
┌─────────────────────────────────────┐
│  Grafana Dashboard                  │
│  - Visualizes metrics               │
│  - Provides interactive filters     │
│  - Generates insights               │
└─────────────────────────────────────┘

Alternative path (CLI):
┌─────────────────────────────────────┐
│  dre node-rewards command           │
│  - Queries Node Rewards Canister    │
│  - Formats data                     │
│  - Outputs to console or CSV        │
└─────────────────────────────────────┘
```

## Metric Naming Convention

The dashboard uses metrics that follow this pattern:

### Provider Metrics
```
latest_nodes_count{provider_id="<principal>"}
latest_total_base_rewards_xdr_permyriad{provider_id="<principal>"}
latest_total_adjusted_rewards_xdr_permyriad{provider_id="<principal>"}
```

**From Command Output:**
- `DailyNodeProviderRewards.total_base_rewards_xdr_permyriad`
- `DailyNodeProviderRewards.total_adjusted_rewards_xdr_permyriad`

### Node Metrics
```
latest_original_failure_rate{node_id="<id>", provider_id="<principal>", subnet_id="<id>"}
latest_relative_failure_rate{node_id="<id>", provider_id="<principal>", subnet_id="<id>"}
```

**From Command Output:**
- `NodeMetricsDaily.original_failure_rate`
- `NodeMetricsDaily.relative_failure_rate`
- Exported in `node_metrics_by_*.csv` files

### Subnet Metrics
```
subnet_failure_rate{subnet_id="<id>"}
```

**From Command Output:**
- `DailyResults.subnets_failure_rate`
- Exported in `subnets_failure_rates.csv`

## Command to Dashboard Mapping

### `dre node-rewards ongoing`
**Output:** Daily summary table

**Dashboard Sections:**
- Section 1: Provider Overview → Current state
- Section 2: Provider Rewards Trends → Time series view
- Section 3: Node-Level Metrics → Individual node details

**Key Fields:**
- Day → Time axis in time series
- Base Rewards Total → Panel 201 (blue lines)
- Adjusted Rewards Total → Panel 201 (green lines)
- Adjusted Rewards % → Panel 202
- Nodes → Panel 203
- Assigned → Not directly shown (can be added)
- Underperf → Panel 305 filtered
- Underperf Nodes → Panel 305 details

### `dre node-rewards past-rewards <month> --compare-with-governance`
**Output:** Comparison table with governance data

**Dashboard Sections:**
- Section 1: Provider Overview → Summary comparison
- Section 5: Analysis & Insights → Rankings

**Key Fields:**
- Provider → provider_id label
- Adjusted → Panel 501, 105
- Base → Panel 105
- Adj-Base Diff → Panel 204
- Adj-Base % → Panel 202, 503
- Governance → Not in dashboard (can be added)
- Adj-Gov Diff → Not in dashboard (can be added)
- Adj-Gov % → Not in dashboard (can be added)

### `--csv-detailed-output-path`
**Generated Files:**

1. **`base_rewards.csv`**
   - Not directly visualized (base-level detail)
   - Could add panel for reward type distribution

2. **`base_rewards_type3.csv`**
   - Region-based rewards
   - Could add regional analysis panels

3. **`rewards_summary.csv`**
   - Maps directly to Section 2 time series

4. **`node_metrics_by_day.csv`**
   - Maps to Section 3 time series (301, 302)

5. **`node_metrics_by_node.csv`**
   - Maps to Panel 305 table

6. **`node_metrics_by_performance_multiplier.csv`**
   - Could add dedicated performance multiplier panel

7. **`subnets_failure_rates.csv`**
   - Maps to Section 4 (401, 402, 404)

## Missing Metrics in Current Dashboard

These are available in the command output but not yet in the dashboard:

### From CSV Exports
1. **Performance Multiplier** (`node_result.performance_multiplier`)
   - Could add: Time series chart showing multiplier trends
   - Could add: Distribution histogram

2. **Node Status** (Assigned vs Unassigned)
   - Could add: Pie chart showing status distribution
   - Currently in `node_status` CSV field

3. **Blocks Proposed/Failed** 
   - `num_blocks_proposed`
   - `num_blocks_failed`
   - Could add: Time series of block production
   - Could add: Block failure rate chart

4. **Extrapolated Failure Rate** (for unassigned nodes)
   - `extrapolated_failure_rate`
   - Could add: Separate panel for unassigned nodes

5. **Node Reward Type** and **Region**
   - `node_reward_type`
   - `region`
   - Could add: Regional distribution analysis
   - Could add: Reward type breakdown

6. **DC ID** (Data Center)
   - `dc_id`
   - Could add: DC-level aggregations
   - Could add: Geographic distribution

7. **Base Rewards Type3 Data**
   - Region-based averages
   - Coefficient analysis
   - Could add: Regional comparison panels

### From Governance Comparison
8. **Governance Rewards**
   - Available when using `--compare-with-governance`
   - Not in current metrics export
   - Would require extending the metrics exporter

## Future Enhancements

### Priority 1 (High Value)
1. **Performance Multiplier Visualization**
   ```promql
   # If exported as metric
   node_rewards_performance_multiplier{node_id, provider_id}
   ```
   - Time series by node
   - Distribution histogram
   - Color-coded by threshold

2. **Block Production Metrics**
   ```promql
   node_rewards_blocks_proposed{node_id}
   node_rewards_blocks_failed{node_id}
   node_rewards_blocks_failure_rate{node_id}
   ```
   - Block production rate
   - Block failure rate trends
   - Correlation with rewards

3. **Node Status Tracking**
   ```promql
   node_rewards_node_status{node_id, status}
   ```
   - Assigned vs unassigned distribution
   - Status change tracking

### Priority 2 (Medium Value)
4. **Regional Analysis**
   - Rewards by region
   - Regional performance comparison
   - Region-based efficiency

5. **Data Center Insights**
   - DC-level aggregations
   - DC failure rate correlation
   - Geographic distribution map

6. **Reward Type Analysis**
   - Type distribution
   - Type-based performance
   - Type-based rewards comparison

### Priority 3 (Nice to Have)
7. **Governance Comparison**
   - Requires extending metrics export
   - Side-by-side comparison
   - Discrepancy analysis

8. **Historical Analysis**
   - Month-over-month comparison
   - Trend analysis
   - Seasonality detection

9. **Predictive Panels**
   - Forecast rewards based on current performance
   - Anomaly detection
   - Alert recommendations

## Implementation Notes

### Variable System
The dashboard uses Grafana's template variables for dynamic filtering:

```json
{
  "name": "provider",
  "query": "label_values(latest_nodes_count, provider_id)",
  "multi": true,
  "includeAll": true
}
```

This allows:
- Multi-select provider filtering
- Cascading filters (provider → node)
- URL-based filter sharing

### Color Coding Standards
- **Green**: Good performance (>98% efficiency, <5% failure)
- **Yellow**: Warning (95-98% efficiency, 5-10% failure)
- **Red**: Critical (<95% efficiency, >10% failure)
- **Blue**: Base/reference values
- **Purple**: Aggregated totals

### Query Optimization
- Use `instant: true` for table queries (snapshot data)
- Use rate/range queries for time series
- Apply filters early in query: `{provider_id=~"$provider"}`
- Use recording rules for complex calculations (if needed)

### Responsive Design
- Panels arranged in 24-column grid
- Most panels span 12 columns (half-width)
- Tables and wide charts span 24 columns (full-width)
- Row headers for logical grouping

## Testing Checklist

- [ ] Import dashboard successfully
- [ ] Configure Prometheus data source
- [ ] Verify all panels load data
- [ ] Test provider filter
- [ ] Test node filter
- [ ] Test subnet filter
- [ ] Verify time range selector works
- [ ] Test refresh functionality
- [ ] Check table sorting
- [ ] Verify color thresholds
- [ ] Test panel links (if any)
- [ ] Export dashboard JSON
- [ ] Verify queries in Prometheus directly

## Deployment Steps

1. **Ensure Metrics Are Exported**
   ```bash
   # Check if metrics are available
   curl http://node-rewards-canister/metrics | grep latest_
   ```

2. **Configure Prometheus Scraping**
   ```yaml
   scrape_configs:
     - job_name: 'node-rewards'
       static_configs:
         - targets: ['node-rewards-canister:port']
   ```

3. **Import Dashboard**
   - Use Grafana UI import
   - Or provision via file

4. **Set Up Alerting** (Optional)
   - Create alert rules based on queries
   - Configure notification channels

5. **Share with Team**
   - Export dashboard URL
   - Document access procedures
   - Set up appropriate permissions

## Maintenance

### Regular Tasks
- Review and update thresholds based on network performance
- Add new panels as metrics are added
- Optimize slow queries
- Update documentation

### When to Update Dashboard
- New metrics added to canister
- Performance thresholds change
- New analysis requirements
- User feedback

## Support & Resources

- **Grafana Docs**: https://grafana.com/docs/
- **PromQL Guide**: https://prometheus.io/docs/prometheus/latest/querying/basics/
- **Dashboard JSON Schema**: https://grafana.com/docs/grafana/latest/dashboards/json-model/
- **Source Code**: `rs/cli/src/commands/node_rewards/mod.rs`

## Conclusion

This dashboard provides comprehensive visualization of node provider rewards data, mapping closely to the CLI command output while adding time-series analysis, filtering, and interactive exploration capabilities. The modular structure makes it easy to extend with additional panels and metrics as needed.

