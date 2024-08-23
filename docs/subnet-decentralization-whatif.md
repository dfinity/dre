# What-If Analysis of Subnet Decentralization

The `WhatifDecentralization` subcommand in the DRE tool allows users to perform "what-if" scenarios on the membership of a subnet and see the effect that the membership change would have on the subnet decentralization, *without actually applying them*, enabling better decision-making and risk assessment.

## Command Structure

The `whatif-decentralization` command is a subcommand of the `subnet` command. The command accepts various parameters that specify the nodes to be added or removed, as well as specifying the subnet on which the analysis is performed.

## Usage

```bash
subnet whatif-decentralization <SUBNET_ID> [--add-nodes <node-id...>] [--remove-nodes <node-id...>] [--subnet-nodes-initial <node-id...>]
```

##### Parameters:

-   **`<SUBNET_ID>`**: The ID of the subnet where the analysis is performed. This parameter is required.

    Example: `tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe`

-   **`--add-nodes`**: A list of node IDs that you want to simulate adding to the subnet.

    Example:

    `--add-nodes yahq2-6rnmm-n7ubm-q76zd-256dl-5f7k6-jxx5l-njyo2-hl7tk-sqcet-6ae tn2ne-tskw6-dfk3n-urdmd-krtq6-tcebq-dx2xr-kokq7-eood7-fadkg-5qe`

    would show the impact on subnet decentralization if we add nodes `yahq2` and `tn2ne` to the subnet.

-   **`--remove-nodes`**: A list of node IDs that you want to simulate removing from the subnet.

    `--remove-nodes gvm7l-ds4n6-vkyjn-gwalp-3vdo6-qfmq7-pxhu4-zvqcu-ozvb7-qz3gr-vqe	 w2sev-mtuls-aa7m5-cdjgi-5vipg-2jn7i-4awvf-o6suu-73ebs-sw5db-kqe`

    would show the impact on subnet decentralization if we remove nodes `gvm7l` and `w2sev` from the subnet.

-   **`--subnet-nodes-initial`**: A list of node IDs representing the initial state of the subnet. This can be used to override the current list of nodes in the subnet for the purpose of analysis.

    Example:

    `--subnet-nodes-initial ncr4b-rasb7-tueb3-n4uos-5nxou-3wbxv-xmyt3-wfdsd-vu4b6-5x3cp-aqe ouffe-miylc-6zcwl-afv2d-lai62-qwzns-xtlji-p7pu2-qkx2e-x72y2-sqe	tm3pc-2bjsx-hhv3v-fsrt7-wotdj-nbu3t-ewloq-uporp-tacou-lupdn-oae	`

    Note that it is necessary to provide the complete list of nodes in the subnet, so most likely you will need to provide 13 or more nodes in the list.

Please note that the number of nodes in the subnet should typically stay unchanged.

#### Example Usage

1.  **Querying decentralization of an existing subnet**:

    ```bash
    dre subnet whatif tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe
    ```

1.  **Adding and Removing Nodes from a Subnet**: To simulate the effect of adding nodes `node4` and `node5` and removing nodes `node6` and `node7` from subnet `subnet123`, use the following command:

    ```bash
    subnet whatif-decentralization subnet123 --add-nodes node4 node5 --remove-nodes node6 node7
    ```

2.  **Specifying Initial Nodes**: If you want to override the current nodes in the subnet with a custom set for the analysis:

    ```bash
    subnet whatif-decentralization subnet123 --subnet-nodes-initial node1 node2 node3 node4 node5 --add-nodes node6 --remove-nodes node2
    ```

#### How It Works

-   The command creates a `ChangeSubnetMembershipPayload` that represents the proposed changes.
-   The specified `SUBNET_ID` is used as the target subnet for the analysis.
-   If the `subnet-nodes-initial` is specified, the analysis uses this custom list of nodes as the starting point; otherwise, it uses the current nodes in the subnet.
-   The `decentralization_change` function then performs the analysis based on the information from the NNS registry, simulating the removal and addition of nodes, and prints the results.
