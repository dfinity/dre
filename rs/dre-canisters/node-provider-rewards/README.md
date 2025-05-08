# Node provider rewards canister

## Suggested Prometheus alert formulas

*Add the job label to the metrics in order to distinguish them from other jobs with similar metrics.*

```yaml
- alert: NodeProviderRewardsSyncFailure
  expr: last_sync_end_timestamp_seconds != last_sync_success_timestamp_seconds
  annotations:
    summary: Canister has failed to sync provider rewards since {{$value | humanizeTimestamp}}.
```

```yaml
- alert: NodeProviderRewardsSyncStalled
  expr: |
     last_sync_start_timestamp_seconds > last_sync_end_timestamp_seconds
     unless
     last_sync_end_timestamp_seconds != last_sync_success_timestamp_seconds
  for: 5m
  annotations:
    summary: Sync of provider rewards has been stalled for 15 minutes.
```

```yaml
- alert: NodeProviderRewardsQueryCallFailed
  expr: |
     query_call_success == 0
  annotations:
    summary: Query call {{$labels.method}} failed to be measured.
```

```yaml
- alert: NodeProviderRewardsQueryCallDangerouslyIntensive
  expr: |
     query_call_instructions / <insert 80% of current IC instruction limit limit for query calls> > 0.9 
  annotations:
    summary: Query call {{$labels.method}} is consuming more than {{ $value | humanizePercentage }} of the query call instruction limit.
```

```yaml
- alert: NodeProviderRewardsQueryCallResponseDangerouslyLarge
  expr: |
     query_call_instructions / <insert 80% of current IC limit for query call response in bytes> > 0.9 
  annotations:
    summary: Query call response for method {{$labels.method}} is producing more than {{ $value | humanize1024 }} bytes, close to the response size limit.
```
