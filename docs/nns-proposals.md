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

### Removing nodes from a subnet

There are situations where nodes need to be removed from subnets, such as for maintenance. Subnets are expected to maintain a certain size (number of nodes). Therefore, nodes cannot simply be removed from a subnet; new nodes must be added back into the subnet to maintain the required number.

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

### Removing nodes from the registry

Here is an example where we remove all AW1 nodes from the registry, for redeployment. Note that the nodes should already be removed from the subnet(s), so they should in unassigned state (awaiting subnet).
You can get the node status by checking [the public dashboard](https://dashboard.internetcomputer.org/center/aw1) or with `dre registry` and then analyzing the generated JSON from the command.

After all relevant nodes are removed from their subnets, you can also remove them from the registry with:

```bash
dre nodes remove aw1 --motivation "Removing AW1 nodes for redeployment"
```
