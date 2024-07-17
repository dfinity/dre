
# Get untrusted metrics from Node Metrics canister

## Introduction

Untrusted Node Metrics retrieval offers an alternative approach to accessing node performance data, relying on a canister that collects these metrics instead of quering the management canister of each subnet directly.

This method allows users to fetch node metrics dating back to May 18, 2024, providing an historical view compared to the trustworthy method, which only offers data from the past month.

The key drawback of quering untrusted metrics is that it introduces an intermediary, the canister responsible for data aggregation, which should NOT be considered trustworthy.

Despite these concerns, the extended temporal coverage can be valuable for certain analytical purposes. Additionally, querying the node metrics canister is cheaper because it allows for a query call instead of an update call and does not require a wallet canister.

This entire process is shown in the following diagram:

```mermaid
%%{init: {'theme':'forest'}}%%
graph TD
    subgraph "Subnet 1"
        S1["Consensus"] -->|Produces Trustworthy Data| M1["Management Canister 1"] --> M4["Node Metrics Canister"]
    end
    subgraph "Subnet 2"
        S2["Consensus"] -->|Produces Trustworthy Data| M2["Management Canister 2"]
    end
    subgraph "Subnet 3"
        S3["Consensus"] -->|Produces Trustworthy Data| M3["Management Canister 3"]
    end
    M2 --> M4
    M3 --> M4
    M4 --> DRE["DRE tool (open source)"]
    DRE --> User
    User --> |Analyze & Process Data| F["Node Metrics"]


    style S1 fill:#f9f,stroke:#333,stroke-width:2px
    style S2 fill:#f9f,stroke:#333,stroke-width:2px
    style S3 fill:#f9f,stroke:#333,stroke-width:2px
    style DRE fill:#ff9,stroke:#333,stroke-width:2px
    style F fill:#9ff,stroke:#333,stroke-width:2px
```

### Using the cli

You can obtain the DRE tool by following the instructions from [getting started](../getting-started.md)

To test out the command you can run the following command

```bash
dre node-metrics from metrics-canister <start-at-timestamp> [<subnet-id>...]
```

??? tip "Explanation of the arguments"
    3. `start-at-timestamp` - used for filtering the output. To get all metrics, provide 0
    4. `subnet-id` - subnets to query, if empty will provide metrics for all subnets

# Example use

Here are some real-world examples of how metrics can be retrieved:

```bash
dre node-metrics from metrics-canister 0 > data.json
```
