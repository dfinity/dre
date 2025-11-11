# Discrete Daily Metrics - Aggregation Guide

## Overview

When working with **discrete daily metrics** (where each day is a separate data point rather than a continuous gauge), you need specific aggregation strategies to properly display totals and trends.

## Key Concepts

### Discrete vs Continuous Metrics

- **Discrete**: Each day has a separate metric value (e.g., rewards for 2025-01-01, rewards for 2025-01-02, etc.)
- **Continuous**: A gauge that continuously updates (e.g., current temperature, current node count)

For discrete metrics, you need to **sum across time** to get totals.

## PromQL Query Patterns for Discrete Daily Metrics

### 1. **Sum All Values Across Time Range**

```promql
sum by (provider_id) (
  sum_over_time(latest_total_adjusted_rewards_xdr_permyriad{provider_id=~"$provider"}[$__range])
)
```

**Use case**: Total rewards for a provider over the selected time period
**How it works**: 
- `sum_over_time()` adds up all data points within the time range
- `sum by (provider_id)` groups results by provider
- `[$__range]` uses the dashboard's selected time range

### 2. **Average Daily Value**

```promql
avg_over_time(
  latest_total_adjusted_rewards_xdr_permyriad{provider_id=~"$provider"}[$__range]
)
```

**Use case**: Average daily rewards
**How it works**: Calculates the mean of all daily values

### 3. **Latest Value (Most Recent Day)**

```promql
last_over_time(
  latest_nodes_count{provider_id=~"$provider"}[$__range]
)
```

**Use case**: Get the most recent node count
**How it works**: Returns the last data point in the time range

### 4. **Count of Data Points (Days with Data)**

```promql
count_over_time(
  latest_nodes_count{provider_id=~"$provider"}[$__range]
)
```

**Use case**: How many days have recorded data
**How it works**: Counts the number of data points

### 5. **Daily Rate of Change**

```promql
deriv(
  latest_total_adjusted_rewards_xdr_permyriad{provider_id=~"$provider"}[1d]
)
```

**Use case**: How fast rewards are growing per day
**How it works**: Calculates the derivative (rate of change)

### 6. **Min/Max Over Time Range**

```promql
# Minimum daily value
min_over_time(
  latest_total_adjusted_rewards_xdr_permyriad{provider_id=~"$provider"}[$__range]
)

# Maximum daily value
max_over_time(
  latest_total_adjusted_rewards_xdr_permyriad{provider_id=~"$provider"}[$__range]
)
```

### 7. **Cumulative Total Across Multiple Metrics**

```promql
sum by (provider_id) (
  sum_over_time(latest_total_base_rewards_xdr_permyriad{provider_id=~"$provider"}[$__range])
) 
+ 
sum by (provider_id) (
  sum_over_time(latest_total_adjusted_rewards_xdr_permyriad{provider_id=~"$provider"}[$__range])
)
```

**Use case**: Total of multiple reward types combined

### 8. **Efficiency Calculation with Aggregation**

```promql
(
  sum by (provider_id) (sum_over_time(latest_total_adjusted_rewards_xdr_permyriad{provider_id=~"$provider"}[$__range]))
  / 
  sum by (provider_id) (sum_over_time(latest_total_base_rewards_xdr_permyriad{provider_id=~"$provider"}[$__range]))
) * 100
```

**Use case**: Overall efficiency percentage across all days

## Updated Dashboard Queries

The dashboard has been updated with these queries:

### Total Nodes (Latest)
```promql
sum(last_over_time(latest_nodes_count[$__range]))
```

### Total Adjusted Rewards (Cumulative)
```promql
sum(sum_over_time(latest_total_adjusted_rewards_xdr_permyriad[$__range]))
```

### Provider Summary Table
All metrics now use `sum_over_time()` to aggregate daily values:
- **Base Total**: `sum by (provider_id) (sum_over_time(...[$__range]))`
- **Adjusted Total**: `sum by (provider_id) (sum_over_time(...[$__range]))`
- **Difference**: Calculated from aggregated sums
- **Efficiency**: Ratio of aggregated values

## Important Variables

- **`$__range`**: Grafana's built-in variable for the selected time range
  - Automatically adjusts when you change the time picker
  - Examples: "7d", "30d", "now-6h to now"

- **`$provider`**: Custom variable for filtering providers
  - Supports regex patterns
  - Multi-select enabled

## Time Range Selector Impact

When you change the time range in Grafana:
- **Last 7 days**: Sums data from 7 discrete daily points
- **Last 30 days**: Sums data from 30 discrete daily points
- **Last 3 months**: Sums data from ~90 discrete daily points

The queries automatically adjust based on your selection!

## Troubleshooting

### Query Returns No Data
1. Check if metrics exist: Go to Explore → Run `{provider_id=~".*"}`
2. Verify time range: Ensure data exists for the selected period
3. Check metric names: Ensure they match exactly

### Values Seem Too High/Low
1. Check if you're double-counting: Make sure `sum_over_time()` is appropriate
2. Verify the time range: A longer range will have higher sums
3. Check for gaps: Use `count_over_time()` to see how many data points exist

### Efficiency > 100%
This usually means the base and adjusted metrics aren't properly aligned. Check:
- Both metrics have the same time series
- The `joinByField` transformation is working correctly
- No duplicate data points

## Best Practices

1. **Always use `sum_over_time()`** for aggregating rewards/totals
2. **Use `last_over_time()`** for getting the latest count/state
3. **Use `avg_over_time()`** for calculating daily averages
4. **Group with `by (provider_id)`** to maintain per-provider granularity
5. **Test queries in Explore** before adding to dashboards

## Example: Creating a New Panel

To create a panel showing "Total Rewards Growth Over Time":

```json
{
  "targets": [
    {
      "expr": "sum by (provider_id) (sum_over_time(latest_total_adjusted_rewards_xdr_permyriad{provider_id=~\"$provider\"}[$__interval:1d]))",
      "legendFormat": "{{provider_id}}",
      "refId": "A"
    }
  ]
}
```

This will show a cumulative line chart of rewards accumulation.

