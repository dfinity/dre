#!/usr/bin/env python3
#
# Interface with ic-admin
#
import functools
import ipaddress
import json
import logging
import pathlib
import re
import subprocess
import typing
from multiprocessing import cpu_count
from multiprocessing import Pool

from tenacity import retry
from tenacity import stop_after_attempt
from tenacity import wait_exponential

from pylib.ic_deployment import IcDeployment
from pylib.ic_utils import download_ic_executable


class IcAdmin:
    """Interface with the ic-admin utility."""

    def __init__(self, deployment: typing.Optional[IcDeployment] = None, git_revision: str = ""):
        """Create an object with the specified ic-admin path and NNS URL."""
        if git_revision:
            self.ic_admin_path = download_ic_executable(git_revision=git_revision, executable_name="ic-admin")
        else:
            self.ic_admin_path = pathlib.Path("ic-admin")
        if not deployment:
            deployment = IcDeployment("mainnet")
        self.deployment = deployment
        self.nns_url = deployment.get_nns_url()

    @retry(
        reraise=True,
        stop=stop_after_attempt(5),
        wait=wait_exponential(multiplier=1, min=2, max=10),
    )
    def _ic_admin_run(self, *cmd):
        cmd = [self.ic_admin_path, "--nns-url", self.nns_url, *cmd]
        cmd = [str(a) for a in cmd]
        logging.info("$ %s", cmd)
        return subprocess.check_output(cmd)

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

    def get_subnet(self, subnet_num):
        """Query the subnet data."""
        logging.debug("NNS %s: get-subnet %s", self.nns_url, subnet_num)
        return json.loads(self._ic_admin_run("get-subnet", str(subnet_num)))

    @functools.lru_cache(maxsize=32)
    def _get_subnet_list(self):
        logging.debug("NNS %s: get-subnet-list", self.nns_url)
        return json.loads(self._ic_admin_run("get-subnet-list"))

    def get_subnets(self):
        """Query the network topology and extract subnets."""
        logging.debug("NNS %s: getting the subnets", self.nns_url)
        return self.get_topology()["topology"]["subnets"]

    def get_subnet_replica_versions(self):
        """Query the network topology and extract subnets and their replica versions."""
        logging.debug("NNS %s: getting the subnet versions", self.nns_url)
        subnet_list = self._get_subnet_list()
        with Pool(cpu_count()) as pool:
            # for each subnet number get the subnet version
            subnets_versions = pool.map(self.get_subnet_replica_version, range(len(subnet_list)))
            # subnets_versions is now an array of versions: each subnet sequentially
            # construct and return a dict of {subnet_id: version}
            return {k: v for k, v in zip(subnet_list, subnets_versions)}

    def get_subnet_replica_version(self, subnet_num):
        """Query the NNS and extract the replica version for the provide subnet_num."""
        result = self.get_subnet(subnet_num)
        return result["records"][0]["value"]["replica_version_id"]

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
        self._ic_admin_run("get-subnet-public-key", "0", out_filename)


if __name__ == "__main__":
    # One can run some simple one-off tests here, e.g.:
    ic_admin = IcAdmin()
    print(ic_admin.get_subnet_replica_versions())
