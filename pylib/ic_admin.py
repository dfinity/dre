#!/usr/bin/env python3
#
# Interface with ic-admin
#
import functools
import ipaddress
import json
import logging
import os
import pathlib
import re
import subprocess

NNS_URL = os.environ.get("NNS_URL") or "http://[2a00:fb01:400:100:5000:5bff:fe6b:75c6]:8080"


class IcAdmin:
    """Interface with the ic-admin utility."""

    def __init__(self, ic_admin_path: pathlib.Path = pathlib.Path("ic-admin"), nns_url: str = NNS_URL):
        """Create an object with the specified ic-admin path and NNS URL."""
        self.ic_admin_path = ic_admin_path
        self.nns_url = nns_url

    def _ic_admin_run(self, *cmd):
        return subprocess.check_output([self.ic_admin_path, "--nns-url", self.nns_url, *cmd])

    @functools.lru_cache(maxsize=32)
    def get_topology(self):
        """Get the network topology."""
        logging.info("Querying the network topology")
        return json.loads(self._ic_admin_run("get-topology"))

    @functools.lru_cache(maxsize=32)
    def get_node_ids(self):
        """Query the network topology and extract all node ids."""
        logging.debug("NNS %s: getting the node IDs", self.nns_url)
        node_ids = {}
        topology = self.get_topology()["topology"]
        for n in topology["unassigned_nodes"]:
            node_ids[n["node_id"]] = "unassigned"
        for subnet_id, subnet in topology["subnets"].items():
            for record in subnet["records"]:
                for member in record["value"]["membership"]:
                    node_ids[member] = subnet_id
        return node_ids

    def get_subnets(self):
        """Query the network topology and extract subnets."""
        logging.debug("NNS %s: getting the subnets", self.nns_url)
        return self.get_topology()["topology"]["subnets"]

    def get_subnet_replica_versions(self):
        """Query the network topology and extract subnets and their replica versions."""
        logging.debug("NNS %s: getting the subnet versions", self.nns_url)
        result = {}
        for subnet_id, subnet in self.get_subnets().items():
            for record in subnet["records"]:
                result[subnet_id] = record["value"]["replica_version_id"]
        return result

    def _get_node(self, node_id):
        logging.debug("NNS %s: getting node info: %s", self.nns_url, node_id)
        return self._ic_admin_run("get-node", node_id)

    def node_get_ipv6(self, node_id):
        """Return the IPv6 address for the provided node ID."""
        r = re.search('ip_addr: "([0-9a-fA-F:]+)"', self._get_node(node_id).decode("utf8"))
        ipv6 = ipaddress.ip_address(r.group(1))
        logging.debug("Extracted node %s ipv6 address: %s", node_id, ipv6)
        return ipv6.compressed

    def get_nns_public_key(self, out_filename):
        """Save the NNS public key in the specified out_filename."""
        if not os.path.exists(out_filename):
            self._ic_admin_run("get-subnet-public-key", "0", out_filename)


if __name__ == "__main__":
    ic_admin = IcAdmin()
    print(ic_admin.node_get_ipv6("7ev5g-lergp-e7ilj-bgucl-qpgwi-6bpjo-itonj-k3aqp-7zios-mkuft-vqe"))
