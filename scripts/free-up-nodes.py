#!/usr/bin/env python3
"""
Automation script for removing nodes from their subnets to free them up for redeployment.

After removing the provided list of nodes from their subnets, the node provider can
redeploy them with IPv4 configuration, in addition to the usual IPv6 configuration.

The script fetches the JSON data of nodes from the internal dashboard,
and groups the nodes by their subnet ids, since we can only submit one proposal
per subnet at a time.

The script then iterates over each subnet, and it replaces the nodes in that subnet.
The script uses the 'dre' command-line tool to replace the nodes.
If there is more than one node in a subnet, all nodes are replaced at once.
"""
import subprocess
import time

import requests

# Node names
nodes = """
bu1-dll01 bu1-dll02
lj1-dll01 lj1-dll02
mb1-dll01 mb1-dll02
pl1-dll01 pl1-dll02
aw1-dll01 aw1-dll02
mu1-dll01 mu1-dll02
hu1-dll01 hu1-dll02
sj2-dll01 sj2-dll02
zh3-dll01 zh3-dll02
zh4-dll01 zh4-dll02
st1-dll01 st1-dll02
ty1-dll01 ty1-dll02
ty2-dll01 ty2-dll02
ty3-dll01 ty3-dll02
""".split()

# Fetching the nodes JSON data
response = requests.get("https://dashboard.internal.dfinity.network/api/proxy/registry/mainnet/nodes")
nodes_data = response.json()

# For each node in the list, find the subnet it's in, and group nodes by subnet ids
print("Node label\tnode id\tnode subnet:")
subnets = {}
for node in nodes:
    for entry in nodes_data.values():
        if entry["hostname"] == node and entry.get("subnet_id"):
            node_id = entry.get("principal")
            subnet_id = entry.get("subnet_id")
            print(node, node_id, subnet_id)
            subnet_nodes = subnets.get(subnet_id, [])
            subnet_nodes.append(node_id)
            subnets[subnet_id] = subnet_nodes

# Sorting and printing unique subnet IDs
for subnet, subnet_nodes in sorted(subnets.items(), key=lambda e: e[0]):
    # Skip subnets with ID less than 'shefu',
    # e.g. if you already submitted proposals for previous subnets
    # if subnet < "shefu":
    #    continue
    print("In subnet %s replacing node(s): %s" % (subnet, subnet_nodes))
    subprocess.check_call(
        [
            "dre",
            "subnet",
            "replace",
            "--exclude",
            *nodes,
            "--motivation",
            (
                "Replacing nodes in the subnet to redeploy them with IPv4 configuration in addition to IPv6"
                if len(subnet_nodes) > 1
                else "Replacing node in the subnet to redeploy it with IPv4 configuration in addition to IPv6"
            ),
            "-o1" if len(subnet_nodes) < 2 else "-o0",
            *subnet_nodes,
        ]
    )
    time.sleep(5)
