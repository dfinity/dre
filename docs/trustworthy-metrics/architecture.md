# Trustworthy Node Metrics: Architectural Overview and Design

## Introduction

This document offers a deeper look at the architectural design of the Trustworthy Node Metrics feature on the Internet Computer (IC). It is tailored for IC stakeholders and technical professionals, providing a detailed understanding of both the functional and structural aspects.


## Objectives

The primary goal is to provide clear visibility into the useful work carried out by nodes on the IC. This transparency is a foundational step towards a future feature that will enable the adjustment of node remuneration based on their operational performance and reliability.


## High-Level Architectural Changes

On a high level, planned architectural changes are illustrated in the following figure:

![Architectural changes diagram](image.png)

### Integration with Existing Architecture

The feature is designed to integrate seamlessly with the existing IC infrastructure. The changes primarily involve the Consensus Layer, Message Routing Layer, and the addition of new components to handle metric aggregation and retrieval.

### Expose already existing information to end users

Consensus Layer to expose block maker information, which will be collected and aggregated by the Message Routing Layer, and stored in the replicated state with success and failure metrics of nodes.
- The metrics for the number of successfully proposed blocks and failures thereof are accumulated in the replicated state for node IDs. 
- The state of this accumulation is saved as a snapshot including the last batch just before midnight in a queue of snapshots (in chronologically ascending order).
- Snapshots in the queue are immutable, i.e. the current state is not included.
- There is no guarantee that the snapshots represent whole days or that all days are included since the subnet could have been offline at some point. 


### Data Accessibility
The inclusion of new components to ensure that the metrics are easily accessible for analysis and decision-making processes. This involves the management canister playing a crucial role in fetching and providing these metrics to stakeholders.

- A metrics-fetching function is added to the management canister

  - There is support for querying since particular date

  - The function will return data from the replicated state.

  - See https://github.com/dfinity/interface-spec/pull/215 for more details.

The DRE team provided open source tooling that fetches the metrics from the management canister(s) of all subnets and allows the community members to inspect the metrics in details.

- The metrics retrieved from the IC can be stored in a local file (JSON format), and then further analyzed

- The metrics will be retrieved from all subnets in parallel, whenever possible, to reduce the amount of time needed to fetch them, taking into account the possible increase of the number of subnets in the future.

- See [trustworthy-metrics](./trustworthy-metrics.md)

## Detailed Architectural Diagrams and Data Flow

The high-level and in-depth technical diagrams provide a visual representation of the data flow within the IC architecture with to the implementation of the Trustworthy Node Metrics feature.

```mermaid
graph TD
    subgraph "IC Community"
        F["Community Members\n[verify metrics]"] -->T[DRE Tooling]
        F --> H[Data Analysis Tools]
        H --> J[JSON data with metrics]
        J --> I[Community Insight and Verification of Node Performance]
    end
    subgraph Sx["IC subnet X"]
        T <----> W
        W[Wallet Canister]
        W <-..->|fetch data| D1
        A1[Consensus Layer] -->|Exposes block maker information| B1["Message Routing (MR) Layer"]
        B1 -->|Aggregates and writes| C1(((Replicated State)))
        D1[Management Canister] -->|Trustworthy metrics API| C1
    end
    subgraph Sy["IC subnet Y"]
        W <-.....->|"Fetch data with\ncross-subnet (XNet) calls"| D2
        A2[Consensus Layer] -->|Exposes block maker information| B2["Message Routing (MR) Layer"]
        B2 -->|Aggregates and writes| C2(((Replicated State)))
        D2[Management Canister] -->|Trustworthy metrics API| C2
    end
    subgraph Sz["IC subnet Z"]
        W <-.....->|"Fetch data with\ncross-subnet (XNet) calls"| D3
        A3[Consensus Layer] -->|Exposes block maker information| B3["Message Routing (MR) Layer"]
        B3 -->|Aggregates and writes| C3(((Replicated State)))
        D3[Management Canister] -->|Trustworthy metrics API| C3
    end
```

[Link for online editing with preview](https://mermaid.live/edit#pako:eNqtlFFv2jAQx7_KyU8gQSWSNzRVoqRVQYNVgEbXhAeTHMFqYiPbGWWl373n0JEwqa2K5pdYd3__7-4Xy88sVgmyLks136xhFkQSaJlieQhEbNCHvsrzQgq7i9gh7dZNGLFjAkaYL1GbKJLhb9RitYMcrRaxWURsAe325SwMJtcwUyoTMl3UbFwSbsOAWw49ybOdEabUmZrqtlQNw-H0xxgSJ90Ku65qHIXDUjgIq84G0oh0bYHLBH661kTMrVAS1ArGNDvcoV4pnXMZ45sRyuQfDNOnsCRBAYkW7mmoquYMvrXbruy8is3DOc8ykva5FMairunnpL-4aF_uV2jjdTnOHoJOJeh1qH9pUJrCwHe-o9NurP3100YZNLDMVPwIOX9EDUKWzbuJ9nDVoTZHaAxPESaqsMQaGqNJ8-By0vVVp_TspanGlFuydYS2WtB2D_1Oo9GY4CZztDCBqaVPs9mszgedcMQlFcpR1sYsTWe6MHartF0f7wH07gbO9l3CuxPCv056PRC7KKFF7OaIrbwFdOdirYxpvx1t3I_RNiEm_iZiRNarkfXOJet9gaz3IVnvU7LeOWS9d8n-OSH78B_J-jWy_rlk_S-Q9T8k639K1j-HrF8jy1osR-pdJPRmPrtExOya_CLWpW2CK15k1r2TLyTlhVXTnYxZ1-oCW6zYEFsMBKc_k7PuimeGopgIq_To8A6Xz3GLbbh8UOqv5uUVRR7Msg)

## Changes in the Public Specification

Addition of the `node_metrics` Interface: This involves updating the existing public spec to include a new `node_metrics` interface that will provide detailed metrics about node performance.

This new interface is marked as experimental, which means that end users should not count on it being permanently being present without changes.


## Security and Reliability Considerations

The feature requires the use of a wallet canister, in order to prevent abuse. Each request for fetching metrics will be charged for, which makes it harder for malicious users to conduct DOS attacks using this interface.

All data is retrieved through `update` calls, in order to prevent a potentially malicious node from providing false data.

## Conclusion

The Trustworthy Node Metrics feature enables the next milestone in the transparency and operational efficiency of the IC. By providing clear insights into node performance, it lays the groundwork for decentralized data-driven decision making, and for future enhancements in node remuneration processes.
