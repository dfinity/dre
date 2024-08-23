Decentralization in the DRE Codebase
====================================

The decentralization in this code is measured using the **Nakamoto Coefficient**.
Nakamoto Coefficient is a key metric used to evaluate the decentralization of decentralized networks by determining the minimum number of independent entities required to disrupt the network's consensus. The coefficient is calculated by analyzing the distribution of nodes across various dimensions or features.

#### **Dimensions / Features Used**:

1.  **Node Providers**: Entities that own or manage nodes.
2.  **Geographic Distribution**: Locations such as country, continent, or city.
3.  **Data Center Ownership**: Ensures that no single data center or its owner dominates.
4.  **Other Custom Features**: Any additional features deemed critical for decentralization.

Each of these features is independently evaluated to understand the distribution of control. The Nakamoto Coefficient is calculated for each feature by determining how many of the most dominant entities are needed to control a critical portion of the network.

#### **Calculation of Nakamoto Coefficients**:

1.  **Feature Analysis**: The code first counts how many nodes are controlled by each entity for each feature.
2.  **Sorting and Accumulation**: These counts for a particular feature are sorted in descending order, and the code accumulates the counts until a threshold (typically one-third of the total nodes) is crossed.
3.  **Coefficient Determination**: The number of entities required to cross this threshold determines the Nakamoto Coefficient for that feature.

**Example:**

Let's consider calculation of the Nakamoto Coefficient for the Node Provider feature in a hypothetical 13-node subnet:

-   Provider A controls 6 nodes.
-   Provider B controls 3 nodes.
-   Provider C controls 2 nodes.
-   Provider D controls 1 node.
-   Provider E controls 1 node.

Next, sort the Node Providers by the number of nodes they control, in descending order:

```yaml
A: 6, B: 3, C: 2, D: 1, E: 1
```

Calculate Total Nodes: summing up, we get 13.

Determine the Threshold: For Nakamoto Coefficient, consider the number of nodes required to control more than 1/3rd of the total nodes:

$$\text{Threshold} = \lceil \frac{13}{3} \rceil = 5$$

Begin accumulating the node counts from the top of the sorted list until the threshold is met.

Accumulate Nodes: `A: 6` -- Threshold is already exceeded with the Provider A: therefore the Nakamoto Coefficient is 1

**Result**: The Nakamoto Coefficient for the Node Provider feature is 1, meaning that a single Node Provider (Provider A) can disrupt the network by controlling more than 1/3rd of the nodes.

**Example with a Higher Coefficient:**

Suppose the distribution was:
```yaml
A: 4, B: 4, C: 3, D: 2
```
Accumulate nodes: `A + B = 4 + 4 = 8 (exceeds the threshold, Nakamoto Coefficient is 2)`

**Result**: Here, the Nakamoto Coefficient is 2, as at least two providers are needed to exceed the threshold.

#### **Comparison of Subnet Configurations**:

When comparing two subnet configurations, the code evaluates the Nakamoto Coefficient across all features before and after a change. The aim is to maximize the lowest Nakamoto Coefficient (minimizing the risk of centralization) while considering other factors like geographic diversity and feature balance.

### Exhaustive Search vs. Heuristic Approach

#### **Exhaustive Search:**

When considering the addition or removal of nodes in a subnet, an exhaustive search would evaluate every possible combination of available nodes. The goal is to identify the combination that maximizes the Nakamoto Coefficient. However, the computational complexity grows exponentially as the number of nodes to be replaced increases.

**Example Calculations:**

1.  **Single Node Replacement**:

    -   For 500 available nodes, the number of possible combinations is **500**.
    -   **Nakamoto Coefficient Calculations**: 500
    -   **Comparisons**: 499

2.  **Two Nodes Replacement**:

    -   Possible combinations: $$ \binom{500}{2} = \frac{500 \times 499}{2} = 124,750$$
    -   **Nakamoto Coefficient Calculations**: 124,750
    -   **Comparisons**: 124,749

3.  **Three Nodes Replacement**:

    -   Possible combinations: $$ \binom{500}{3} = \frac{500 \times 499 \times 498}{6} = 20,708,500$$
    -   **Nakamoto Coefficient Calculations**: 20,708,500
    -   **Comparisons**: 20,708,499

4.  **Four Nodes Replacement**:

    -   Possible combinations: $$ \binom{500}{4} = \frac{500 \times 499 \times 498 \times 497}{24} \approx 2.57 \times 10^9$$
    -   **Nakamoto Coefficient Calculations**: 2.57 billion
    -   **Comparisons**: 2.57 billion - 1

5.  **Five Nodes Replacement**:

    -   Possible combinations: $$ \binom{500}{5} = \frac{500 \times 499 \times 498 \times 497 \times 496}{120} \approx 2.57 \times 10^{11}$$
    -   **Nakamoto Coefficient Calculations**: 257 billion
    -   **Comparisons**: 257 billion - 1

As you can see, we very quickly get into a combinatorial explosion, which makes it computationally extremely expensive to perform an exhaustive search. We therefore have to approach a heuristic in general case.

#### **Heuristic Approach:**

Rather than evaluating all possible combinations, the heuristic approach:

1.  **Calculate Nakamoto Coefficients for all Candidates**:

    -   For each available node calculate the Nakamoto coefficients (across all dimensions/features) that we would get if we added the particular node into the subnet.
2.  **Penalty System**:

    - Penalize candidate nodes that do not satisfy essential business rules, such as avoiding centralization in specific regions or data centers.
3.  **Narrow down the candidates with the best Nakamoto coefficients**:
    - From the list of all candidate nodes, compose a list of the candidates that all have the same and maximum Nakamoto coefficients *across all dimensions*

4.  **Deterministic Random Selection**:

    -   Among the top candidates (those that pass the initial selection), a deterministic random process is used to choose the best node(s). If some of the initial nodes are in the top candidates, prefer to keep the initial nodes.

**Example Calculations:**

1.  **Single Node Replacement**:

    -   **Nakamoto Coefficient Calculations**: 500 (same as exhaustive)
    -   **Comparisons**: 500 (same as exhaustive)

2.  **Multiple Nodes Replacement**:

    -   Instead of calculating every combination of candidate nodes, the heuristic picks a replacement node in each iteration, dramatically reducing calculations. For instance:
        -   **Two Nodes**: 500 + 499 = 999
        -   **Three Nodes**: 500 + 499 + 498 = 1497
    -   The cost of calculation grows linearly with the number of replaced nodes (total cost being approximately the number of available nodes * number of replaced nodes)
    -   This is significantly fewer than exhaustive combinations and makes it possible to very quickly replace 10s of nodes.

In summary, the heuristic approach enables the code to efficiently find optimal or near-optimal solutions without the prohibitive cost of exhaustive search. It balances the need for thorough evaluation with the practical limits of computation, making it feasible to manage large, decentralized networks effectively.

If needed in the future, the node selection process can be improved by keeping the best potential replacement candidates (e.g. 10-20 nodes) and only perform exhaustive combinatorial search among those. However, even in that case the number of computations would very quickly grow and would be necessary to approach such a change very carefully.

### Other Areas of Interest

#### **Optimization Limits**:

The system limits the number of node replacements in a single operation to prevent excessive state-sync operations and disruption to the network's state, especially in critical subnets.

#### **Health Considerations**:

Unhealthy or degraded nodes are prioritized for removal, and by default are automatically included into the replacement proposals, to maintain the network's overall health.

#### **Business Rule Enforcement**:

The code enforces strict business rules to ensure that no single entity or feature becomes overly dominant. This includes maintaining a minimum number of DFINITY-owned nodes in each subnet and ensuring geographic diversity.

#### **Network Healing and Resilience**:

The network healing process is designed to identify unhealthy subnets and optimize them by replacing underperforming nodes with those that improve decentralization and resilience. This process is particularly critical for maintaining the security and efficiency of critical subnets like the NNS (Network Nervous System).

Quick Introduction to the Source Code
-------------------------------------

The [lib.rs](https://github.com/dfinity/dre/blob/main/rs/decentralization/src/lib.rs) file in the codebase serves as the main entry point for the module that handles decentralization in the network. Here's a breakdown of its contents:

### Modules

-   **[nakamoto/mod.rs](https://github.com/dfinity/dre/blob/main/rs/decentralization/src/nakamoto/mod.rs)**: Likely contains logic for calculating the Nakamoto Coefficient, a metric to measure decentralization.
-   **[network.rs](https://github.com/dfinity/dre/blob/main/rs/decentralization/src/network.rs)**: Handles operations related to the network, including nodes and subnets.
-   **[subnets.rs](https://github.com/dfinity/dre/blob/main/rs/decentralization/src/subnets.rs)**: Manages subnet-related operations.

### Structs

-   **`SubnetChangeResponse`**: Captures the details of changes made to a subnet, including nodes added or removed, changes in the Nakamoto score, and additional metadata like motivation and comments.

### Key Features:

-   **Nakamoto Score**: The struct utilizes the Nakamoto score to compare the decentralization state before and after changes.
-   **Feature Diff**: Provides a detailed comparison of network features before and after a change, stored in a `BTreeMap`.

### Implementation Details:

-   The struct provides methods to generate responses based on subnet changes and can display these changes, including the impact on decentralization, in a user-friendly format.

### Display Implementation:

-   The `Display` trait is implemented to print detailed information about the changes, including the impact on the Nakamoto coefficient and lists of added or removed nodes. It also provides a tabular view of feature differences, making it easy to see how changes affect network decentralization.

### Nakamoto Coefficient Calculation

Nakamoto coefficient calculation source code: [nakamoto/mod.rs](https://github.com/dfinity/dre/blob/main/rs/decentralization/src/nakamoto/mod.rs).

#### Node Replacement and Selection

The code optimizes for decentralization when replacing nodes by carefully selecting which nodes to add or remove from a subnet.

##### **Node Removal**

The node removal process prioritizes removing nodes belonging to over-represented actors, reducing centralization risk. It calculates how each candidate's removal affects the Nakamoto Coefficient, choosing the node whose removal least negatively impacts the network's decentralization.

##### **Node Addition**

Adding a node follows similar logic. The system seeks to increase decentralization by selecting a node that strengthens the network's distribution across key features. The chosen node typically adds diversity, either by being from an under-represented actor or by being located in a different geographical area.

#### Calculation of Best Candidate Nodes

The code calculates the "best candidate" for node removal and addition by evaluating several criteria:

-   **Impact on Nakamoto Coefficient**: The primary criterion for node replacement is how a node's addition or removal affects the Nakamoto Coefficient. The goal is to maintain or increase the coefficient, ensuring the network remains decentralized.
-   **Controlled Nodes**: The number of nodes controlled by a single actor is assessed, and nodes that would increase this control are deprioritized.
-   **Critical Features**: For critical features (e.g., Node Providers, Countries), the system checks the balance and tries to distribute control evenly across actors.

These checks are implemented in functions like `describe_difference_from` and `nakamoto`, which evaluate and compare different scenarios to ensure optimal decentralization.

### Network Optimization

The `src/network.rs` file is a comprehensive module that manages decentralized subnets in a distributed network. It involves various structures, traits, and implementations that work together to ensure optimal decentralization and resilience against centralization risks. Here's a more detailed analysis of its contents:

#### Key Structures and Their Roles:

1.  **`Node` Structure**:

    -   Represents individual nodes in the network, each identified by a `PrincipalId`.
    -   Stores node features, such as whether a node is owned by DFINITY.
    -   Implements utility methods for feature comparison, display, and conversion from external types (`ic_management_types::Node`).

2.  **`DecentralizedSubnet` Structure**:

    -   Manages subnet configurations, maintaining a list of nodes and tracking changes (additions/removals).
    -   Includes methods for subnet modifications while ensuring compliance with decentralization business rules, such as maintaining a minimal number of DFINITY-owned nodes and geographic diversity.
    -   Calculates the Nakamoto Coefficient to measure decentralization and uses this metric to evaluate the impact of node changes.

3.  **Node Replacement and Subnet Optimization**:

    -   Implements sophisticated logic for selecting nodes to add or remove from subnets, guided by the goal of maximizing decentralization.
    -   **`ReplacementCandidate`**: Evaluates potential nodes for inclusion or removal, considering their impact on the Nakamoto Coefficient and any penalties (e.g., non-decentralization).
    -   Methods like `subnet_with_more_nodes` and `subnet_with_fewer_nodes` handle adding or removing nodes while preserving or enhancing decentralization.

4.  **SubnetChangeRequest Structure**:

    -   Facilitates requests to modify subnets, including adding or removing nodes.
    -   Includes methods to optimize the subnet by balancing the Nakamoto Coefficient, ensuring the network remains decentralized.
    -   Can be used to "rescue" a subnet by reconfiguring its nodes to restore or improve its decentralization metrics.

5.  **Network Healing**:

    -   **`NetworkHealRequest`**: Orchestrates the process of identifying unhealthy nodes within subnets and optimizes the network by replacing these nodes with healthier, more decentralized alternatives.
    -   Prioritizes critical subnets like the NNS (Network Nervous System) during the healing process.
    -   Implements a deterministic yet randomized selection process to avoid biases in node replacement while ensuring consistency in subnet composition.

6.  **Business Rule Enforcement**:

    -   Rigorously enforces business rules for subnets, such as ensuring that no single actor controls too many nodes and that geographic diversity is maintained.
    -   The `check_business_rules` method validates subnet configurations against these rules, issuing penalties for non-compliance that affect node selection during optimizations.

#### Detailed Logic and Features:

1.  **Deterministic Node Selection**:

    -   Ensures that node selection, even when randomized, is deterministic based on the current subnet state. This guarantees that identical inputs yield identical results, crucial for maintaining consistency in a distributed network.

2.  **Penalties and Score Comparisons**:

    -   Nodes are evaluated based on penalties associated with violating business rules, and these penalties influence the selection of nodes during optimization. The code carefully balances penalties with the need to maximize the Nakamoto Coefficient.

3.  **Handling of Critical Subnets**:

    -   Critical subnets, such as those managing key infrastructure like the NNS, receive special attention during the healing process. The code ensures that these subnets are kept robust and decentralized, minimizing risks of centralization.

4.  **Feature Mapping and Evaluation**:

    -   Nodes are categorized by features (e.g., geographic location, data center ownership), and the code evaluates how changes to these features affect the overall decentralization of the network.

### Subnet Health and Node Removal

The `src/subnets.rs` file focuses on managing the health and removal of nodes within subnets. Here's a detailed breakdown:

#### Functions and Structures:

1.  **`unhealthy_with_nodes` Function**:

    -   Identifies subnets that contain unhealthy nodes. It takes a map of subnets and node health statuses, returning a map where each entry is a subnet ID and a list of unhealthy nodes within that subnet.

2.  **`NodesRemover` Struct**:

    -   Manages the removal of nodes from the network based on various criteria.

    **Key Fields**:

    -   `no_auto`: Disables automatic node removal.
    -   `remove_degraded`: Determines whether to remove nodes marked as degraded, in addition to dead nodes.
    -   `extra_nodes_filter`: A list of criteria to filter additional nodes for removal.
    -   `exclude`: A list of features to exclude certain nodes from removal.
    -   `motivation`: The reason or motivation for node removal.

    **`remove_nodes` Method**:

    -   Processes the removal of nodes from the network based on the criteria set in the struct.
    -   Filters nodes based on their health status, subnet association, and whether they match specific filters or exclusion lists.
    -   Nodes are removed for reasons such as being unhealthy, duplicates, or matching specific filters.

#### Detailed Analysis:

1.  **Node Health Management**:

    -   The `unhealthy_with_nodes` function is essential for maintaining the health of the network by identifying and isolating unhealthy nodes within subnets. This function is likely used in broader network management routines to ensure subnets are composed of healthy nodes.

2.  **Node Removal Process**:

    -   The `NodesRemover` struct is designed to handle the complex process of node removal, allowing flexibility in what nodes are removed and why. By incorporating filters, exclusions, and health checks, this struct ensures node removal is performed in a controlled and justified manner.
    -   The `remove_nodes` method is particularly robust, handling various scenarios where nodes might need removal, including automatic removal based on health status, exclusion of specific nodes, and custom filtering.

3.  **Flexibility and Customization**:

    -   The file provides a high degree of customization in managing node health and removal, crucial for maintaining the overall health and decentralization of the network. The ability to filter, exclude, and motivate node removal allows network administrators to tailor the node management process to specific needs and conditions.
