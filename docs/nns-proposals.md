# Submitting NNS proposals

Most of the commands here can be run in multiple ways. Currently we are putting in the effort to make `dre` as useful as possible. As such it provides support for `dry_run` as default and that can be highly beneficial in most scenarios (for eg. if someone is asking you to submit a proposal for them the best practice way is to run a `dry_run` and ask them to double check the command and the payload that would be submitted) and that is why we recommend using `dre` whenever possible. In some use-cases `dre` cannot help you, and that is when you should use whatever tool/script is at hand.

### Get the principal from your HSM

```bash
❯ dfx identity use hsm
Using identity: "hsm".
❯ export DFX_HSM_PIN=$(cat ~/.hsm-pin)
❯ dfx identity get-principal
as4rt-t4nqh-64j36-ubyaa-2c6uz-f2qbm-67llh-ftd3y-epn2e-wzaut-wae
```

### Get the neuron id associated with your HSM

```bash
❯ export DFX_HSM_PIN=$(cat ~/.hsm-pin)
❯ dfx canister --identity=hsm --network=ic call rrkah-fqaaa-aaaaa-aaaaq-cai get_neuron_ids '()'
(vec { 40 : nat64 })
```

### Getting the Mainnet firewall rules

```bash
dre get firewall-rules replica_nodes | jq
```

### Get the Node Rewards Table, used for the Node Provider compensation

```bash
dre get node-rewards-table
{
  "table": {
    "Asia": {
[...]
}
```

### Update the Node Rewards Table

```bash
dre propose update-node-rewards-table --summary-file 2022-12-type3.md --updated-node-rewards "$(cat 2022-12-type3-rewards.json | jq -c)"
```

### Enable the HTTPs outcalls on a subnet

```bash
dre propose update-subnet \
	--features "http_requests" \
	--subnet uzr34-akd3s-xrdag-3ql62-ocgoh-ld2ao-tamcv-54e7j-krwgb-2gm4z-oqe \
	--summary "Enable the HTTPS outcalls feature on the non-whitelisted uzr34 subnet so that the exchange rate canister can query exchange rate data."
```

### Removing node operator principal id

```bash
dre propose remove-node-operators kdqam-hauon-sdvym-42eyg-5wyff-4ywbw-v6iij-2sw2z-bu4rj-ejusn-jae \
    --summary "<An appropriate summary for the proposal, and a link to the forum post for further discussion, if possible>"
```

### Replacing (or removing) nodes in a subnet

There are situations where nodes need to be removed from subnets, such as for maintenance. Subnets are expected to maintain a certain size (number of nodes). Therefore, nodes cannot simply be removed from a subnet; new nodes must be added back into the subnet to maintain the required number.

The `ic-admin` command for this is `propose-to-change-subnet-membership` but this command should NOT be invoked directly from `ic-admin` because it's necessary to consider the effect of node replacement on the subnet decentralization.
The DRE tool will 1) find the optimal replacement for a node the given subnet, and 2) help you to review and submit the required NNS proposal.

Typical usage would be like this:

```bash
dre subnet replace <NODE_ID...>
```

**Note:** It is possible to provide multiple node IDs in the same command line, but they MUST be in the same subnet.

Example usage to remove an unhealthy node from the subnet `tdb26`:

```bash
dre subnet replace --id tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe
```

Example usage to remove healthy nodes from their current subnet:

```bash
dre subnet replace --nodes tm3pc-2bjsx-hhv3v-fsrt7-wotdj-nbu3t-ewloq-uporp-tacou-lupdn-oae 5iihd-fkroy-ow5zp-hlvwz-bsgbl-mecta-kxubm-6adxr-ckcu6-prsus-fqe --motivation "Removing nodes for redeployment. Link to the forum post: https://forum.dfinity.org/...."
```

### Replacing and optimizing nodes in all subnet with unhealthy nodes

It is possible to run a single dre command to a) find all subnets with unhealthy nodes, b) subnet proposals for all subnets with unhealthy nodes, and attempt to replace a few additional nodes to improve optimization.

The invocation is straighforward:

```bash
dre heal
```

Please pay close attention to the output of the command and if possible get someone else to double check the effect on decentralization before confirming the proposal submission.

#### Freeing up (removing from their subnets) nodes from a specific DC

To free up nodes from a specific data center (e.g., bo1) within subnets, follow these steps:

First, [open the public dashboard for the DC in question](https://dashboard.internetcomputer.org/center/bo1) to find a list of subnets that include nodes from the data center. Or, alternatively, you can bypass the public dashboard by querying the registry directly:

```bash
dre registry | jq '.nodes[] | select((.dc_id == "bo1") and (.subnet_id != null))'
```
**Explanation**: This command retrieves a dump of the registry, extracts the list of nodes, and selects only those that belong to the `bo1` data center and have a `subnet_id` set (i.e., not null).

<details>
  <summary><i>Click here to see the output of the above command</i></summary>
  ```json
  {
    "node_id": "4jtgm-ywxcc-xh3o3-x2omx-tgmdm-gobca-agb3a-alvw4-dhmyn-khis6-xae",
    "xnet": {
      "ip_addr": "2600:c0d:3002:4:6801:dfff:fee7:5ae8",
      "port": 2497
    },
    "http": {
      "ip_addr": "2600:c0d:3002:4:6801:dfff:fee7:5ae8",
      "port": 8080
    },
    "node_operator_id": "ut325-qbq5v-fli2f-e2a5h-qapdd-fsuyv-xej2j-ogvux-i3fc2-5nj3a-2ae",
    "chip_id": null,
    "hostos_version_id": "2e269c77aa2f6b2353ddad6a4ac3d5ddcac196b1",
    "public_ipv4_config": null,
    "subnet_id": "nl6hn-ja4yw-wvmpy-3z2jx-ymc34-pisx3-3cp5z-3oj4a-qzzny-jbsv3-4qe",
    "dc_id": "bo1",
    "node_provider_id": "lq5ra-f4ibl-t7wpy-hennc-m4eb7-tnfxe-eorgd-onpsl-wervo-7chjj-6qe",
    "status": "Healthy"
  }
  {
    "node_id": "af7ti-auyik-jfsne-tljmz-6purg-2msmy-jw34z-b4ie3-abk5f-h23xt-zae",
    "xnet": {
      "ip_addr": "2600:c0d:3002:4:6801:19ff:fe8c:de47",
      "port": 2497
    },
    "http": {
      "ip_addr": "2600:c0d:3002:4:6801:19ff:fe8c:de47",
      "port": 8080
    },
    "node_operator_id": "ut325-qbq5v-fli2f-e2a5h-qapdd-fsuyv-xej2j-ogvux-i3fc2-5nj3a-2ae",
    "chip_id": null,
    "hostos_version_id": "2e269c77aa2f6b2353ddad6a4ac3d5ddcac196b1",
    "public_ipv4_config": null,
    "subnet_id": "w4rem-dv5e3-widiz-wbpea-kbttk-mnzfm-tzrc7-svcj3-kbxyb-zamch-hqe",
    "dc_id": "bo1",
    "node_provider_id": "lq5ra-f4ibl-t7wpy-hennc-m4eb7-tnfxe-eorgd-onpsl-wervo-7chjj-6qe",
    "status": "Healthy"
  }
  {
    "node_id": "fd5e4-a2xzl-lxu7m-kjvn6-2arnt-jghro-rdrgx-zvvkd-j2hza-pbwl4-5qe",
    "xnet": {
      "ip_addr": "2600:c0d:3002:4:6801:94ff:fec9:6b",
      "port": 2497
    },
    "http": {
      "ip_addr": "2600:c0d:3002:4:6801:94ff:fec9:6b",
      "port": 8080
    },
    "node_operator_id": "ut325-qbq5v-fli2f-e2a5h-qapdd-fsuyv-xej2j-ogvux-i3fc2-5nj3a-2ae",
    "chip_id": null,
    "hostos_version_id": "2e269c77aa2f6b2353ddad6a4ac3d5ddcac196b1",
    "public_ipv4_config": null,
    "subnet_id": "tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe",
    "dc_id": "bo1",
    "node_provider_id": "lq5ra-f4ibl-t7wpy-hennc-m4eb7-tnfxe-eorgd-onpsl-wervo-7chjj-6qe",
    "status": "Healthy"
  }
  {
    "node_id": "q2ucv-x7dv5-hheao-ocsye-jbg4z-enm75-ss62d-ehqhj-zwwm3-cap5q-tqe",
    "xnet": {
      "ip_addr": "2600:c0d:3002:4:6801:bcff:fee7:4008",
      "port": 2497
    },
    "http": {
      "ip_addr": "2600:c0d:3002:4:6801:bcff:fee7:4008",
      "port": 8080
    },
    "node_operator_id": "ut325-qbq5v-fli2f-e2a5h-qapdd-fsuyv-xej2j-ogvux-i3fc2-5nj3a-2ae",
    "chip_id": null,
    "hostos_version_id": "2e269c77aa2f6b2353ddad6a4ac3d5ddcac196b1",
    "public_ipv4_config": null,
    "subnet_id": "io67a-2jmkw-zup3h-snbwi-g6a5n-rm5dn-b6png-lvdpl-nqnto-yih6l-gqe",
    "dc_id": "bo1",
    "node_provider_id": "lq5ra-f4ibl-t7wpy-hennc-m4eb7-tnfxe-eorgd-onpsl-wervo-7chjj-6qe",
    "status": "Healthy"
  }
  ```
</details>

To further refine the command and group nodes by subnets:
```bash
DC=bo1
dre registry | jq -r '.nodes | map(select((.dc_id == "'$DC'") and (.subnet_id != null))) | group_by(.subnet_id) | map("dre subnet replace --exclude '$DC' --nodes \([.[].node_id] | join(" "))") | .[]'
```

<details>
<summary><i>Click here to see the explanation of the jq command</i></summary>

1.  **`-r` (raw output):**

    -   This option tells `jq` to output raw strings instead of JSON-formatted strings. This is particularly useful when you want to generate command-line strings like the one you're creating.
2.  **`.nodes`:**

    -   This accesses the `nodes` array in the JSON structure.
3.  **`map(select((.dc_id == "bo1") and (.subnet_id != null)))`:**

    -   This filters the nodes array. It keeps only the nodes where `dc_id` is `"bo1"` and `subnet_id` is not `null`. `map` applies this filter to each element of the array.
4.  **`group_by(.subnet_id)`:**

    -   This groups the filtered nodes by their `subnet_id`. The result is an array of arrays, where each inner array contains nodes that share the same `subnet_id`.
5.  **`map("dre subnet replace --exclude '$DC' --nodes \([.[].node_id] | join(" "))")`:**

    -   This `map` applies a transformation to each group (each inner array).
    -   The transformation constructs a string that starts with `"dre subnet replace --exclude '$DC' --nodes "`.
    -   `\([.[].node_id] | join(" "))`:
        -   `.[].node_id` accesses the `node_id` of each node in the current group.
        -   `join(" ")` joins all the `node_id`s into a single string separated by spaces.
    -   The result is a string for each subnet that lists all its nodes.
6.  **`.[]`:**

    -   This unwraps the array of strings created by the previous `map` step into individual strings. This means the output will be a list of commands, one per line.
</details>

<details>
<summary><i>Click here to see the example output</i></summary>
```
dre subnet replace --exclude bo1 --nodes q2ucv-x7dv5-hheao-ocsye-jbg4z-enm75-ss62d-ehqhj-zwwm3-cap5q-tqe
dre subnet replace --exclude bo1 --nodes 4jtgm-ywxcc-xh3o3-x2omx-tgmdm-gobca-agb3a-alvw4-dhmyn-khis6-xae
dre subnet replace --exclude bo1 --nodes fd5e4-a2xzl-lxu7m-kjvn6-2arnt-jghro-rdrgx-zvvkd-j2hza-pbwl4-5qe
dre subnet replace --exclude bo1 --nodes af7ti-auyik-jfsne-tljmz-6purg-2msmy-jw34z-b4ie3-abk5f-h23xt-zae
```
</details>

You would now go through each of these output lines (subnets) and submit proposals. Remember to include a motivation for each proposal. For example:

```bash
dre subnet replace --exclude bo1 --nodes q2ucv-x7dv5-hheao-ocsye-jbg4z-enm75-ss62d-ehqhj-zwwm3-cap5q-tqe --motivation "Removing BO1 nodes for maintenance"
```

### Replacing a Specific Node in a Subnet

The following step-by-step instructions describe how a node in a subnet can be replaced using the DRE tool while maintaining the same number of nodes in the subnet.

#### Prerequisites
- DRE tool installed and configured.
- Knowledge of the node IDs to be replaced.

#### Step-by-Step Instructions

##### 1. Understand the Command Structure
The command to replace a node looks like this:
```bash
❯ dre subnet replace --nodes <NODES_TO_REMOVE> --motivation "<REPLACEMENT_REASON>"
```

##### 2. Example Command
Below is an example command to replace a node in subnet `pjljw`:
```bash
❯ dre subnet replace \
  --nodes z6jp6-245uu-gh3cs-sblcy-f3jmj-s4ngl-v3z4u-lafz2-qudjr-6mbqx-vqe \
  --motivation "Requested by the node operator in order to redeploy all nodes in the DC after 48 months, and switch to a new node operator ID."
```

Note that it's possible to provide multiple node ids in the command above, separated by spaces, as long as they are all in the same subnet.

##### 3. Execute the Command
Run the command to propose the node replacement. The tool will perform a series of checks and outputs similar to the following:

```plaintext
2024-12-27T17:33:31.470Z INFO  dre > Running version 0.5.7-9c513fc1
2024-12-27T17:33:31.541Z INFO  dre::store > Using local registry path for network mainnet: /home/user/.cache/dre-store/local_registry/mainnet
...
2024-12-27T17:33:54.327Z INFO  dre::ic_admin > running ic-admin:
...
```

##### 4. Verify the Proposal Details
Review the details provided by the tool for:
- **Nodes to be removed**
- **Nodes to be added**
- **Impact on decentralization metrics**

##### 5. Confirm the Replacement
If the proposal details look correct, confirm the action when prompted:
```plaintext
Do you want to continue? yes
```

##### 6. Post-Execution Verification
To get the proposal adopted and executed:
- Please answer questions in the forum if there are any for the proposal.

##### 7. Post-Execution Verification
After the replacement process is complete:
- Verify that the new node is active and healthy in the subnet.

#### Example Output
The tool provides a detailed summary of the decentralization impact, as shown below:

```plaintext
Decentralization Nakamoto coefficient changes for subnet `pjljw`:

    node_provider: 5.00 -> 5.00    (+0%)
      data_center: 5.00 -> 5.00    (+0%)
data_center_owner: 5.00 -> 5.00    (+0%)
             area: 5.00 -> 5.00    (+0%)
          country: 4.00 -> 4.00    (+0%)
```

#### Notes
- Always verify the decentralization impact to ensure the subnet remains balanced.
- If any issues arise, consult the DRE documentation or reach out to the appropriate support channels.

#### Additional Resources
- [DRE GitHub Repository](https://github.com/dfinity/dre)
- [Dfinity Forum](https://forum.dfinity.org/)

### Removing nodes from the registry

Here is an example where we remove all AW1 nodes from the registry, for redeployment. Note that the nodes should already be removed from the subnet(s), so they should in unassigned state (awaiting subnet).
You can get the node status by checking [the public dashboard](https://dashboard.internetcomputer.org/center/aw1) or with `dre registry` and then analyzing the generated JSON from the command.

After all relevant nodes are removed from their subnets, you can also remove them from the registry with:

```bash
dre nodes remove aw1 --motivation "Removing AW1 nodes for redeployment"
```
