#!/usr/bin/env python3
"""Interface with the deployment Ansible configuration, e.g. get nodes, ipv6 addresses, etc."""
import json
import os
import subprocess

import yaml

from .ic_utils import find_deployment_env_root
from .ic_utils import repo_root


class IcDeployment:
    """Interface with the deployment Ansible configuration, e.g. get nodes, ipv6 addresses, etc."""

    def __init__(
        self,
        deployment_name: str,
        nodes_filter_include: str = "",
        nns_url_override: str = "",
        decentralized_deployment=False,
    ):
        """Create an object for the given git repo root and deployment name."""
        self._name = deployment_name
        if deployment_name == "staging" and not nodes_filter_include:
            nodes_filter_include = "node_type=(child_nns|app_[0-9]+)"
        self._deployment_env_root = find_deployment_env_root(deployment_name)
        self._inventory_script = self._deployment_env_root.parent.parent / "ansible/inventory/inventory.py"
        self.nodes_filter_include = nodes_filter_include
        self.nns_url_override = nns_url_override
        self.decentralized_deployment = decentralized_deployment

    @property
    def name(self):
        """Return the deployment name."""
        return self._name

    def get_ansible_inventory(self, all_physical_hosts=False):
        """Get an Ansible inventory list for a deployment."""
        env = os.environ.copy()
        env["DEPLOYMENT"] = self._name
        if all_physical_hosts:
            env["INCLUDE_ALL_PHYSICAL_HOSTS"] = "1"
        if self.decentralized_deployment:
            env["DECENTRALIZED_DEPLOYMENT"] = "true"
        output = subprocess.check_output(
            ([
                "ansible-inventory",
            ] if not os.environ['ANSIBLE_INVENTORY_BIN'] else [
                "python3",
                os.environ['ANSIBLE_INVENTORY_BIN'],
            ]) +
            [
                "-i",
                self._deployment_env_root / "hosts",
                "--list",
            ],
            env=env,
        )
        return json.loads(output)

    def get_deployment_nodes_ipv6(self):
        """Get a list of nodes for a deployment, as a dictionary of {node_name: ipv6}."""
        env = os.environ.copy()
        if self.nodes_filter_include:
            env["NODES_FILTER_INCLUDE"] = self.nodes_filter_include
        output = subprocess.check_output(
            [
                self._inventory_script,
                "--deployment",
                self.name,
                "--nodes",
            ]
            + (["--decentralized-deployment"] if self.decentralized_deployment else []),
            env=env,
        )
        return yaml.load(output, Loader=yaml.FullLoader)

    def get_deployment_subnet_nodes(self, subnet_name):
        """Get a list of nodes for a deployment, as a dictionary of {node_name: ipv6}."""
        env = os.environ.copy()
        if self.nodes_filter_include:
            env["NODES_FILTER_INCLUDE"] = self.nodes_filter_include
        output = subprocess.check_output(
            [
                self._inventory_script,
                "--deployment",
                self.name,
                "--list",
            ]
            + (["--decentralized-deployment"] if self.decentralized_deployment else []),
            env=env,
        )
        return json.loads(output)[subnet_name]["hosts"]

    def get_deployment_nodes_hostvars(self):
        """Get the Ansible hostvars for all nodes of a deployment."""
        ansible_inventory = self.get_ansible_inventory()
        deployment_nodes_ipv6 = self.get_deployment_nodes_ipv6()
        nodes_hostvars = {}
        for node in deployment_nodes_ipv6.keys():
            nodes_hostvars[node] = ansible_inventory["_meta"]["hostvars"][node]
        return nodes_hostvars

    def get_nns_url(self):
        """Get the NNS nodes for a deployment."""
        if self.nns_url_override:
            return self.nns_url_override
        if self.name in ["mercury", "mainnet"]:
            return "https://ic0.app/"
        nns_nodes = self.get_deployment_subnet_nodes("nns")
        all_nodes_ipv6 = self.get_deployment_nodes_ipv6()
        nns_node_addr = all_nodes_ipv6[nns_nodes[0]]
        return f"http://[{nns_node_addr}]:8080"

    def get_deployment_physical_hosts(self):
        """Get the physical hosts in the deployment."""
        ansible_inventory = self.get_ansible_inventory(all_physical_hosts=False)
        return ansible_inventory["physical_hosts"]["hosts"]

    @property
    def serial_numbers(self):
        """Return the serial numbers for the machines in the Mercury DCs."""
        serial_numbers_filename = self._deployment_env_root.parent / "serial-numbers.yml"
        all_serials = yaml.load(open(serial_numbers_filename, encoding="utf8"), Loader=yaml.FullLoader)
        return {k: v for k, v in all_serials.items() if k.split("-")[0] in self.mercury_dcs}

    @property
    def mercury_dcs(self):
        """Return a list of Mercury DCs."""
        active_dcs = repo_root / "factsdb/mercury-dcs.yml"
        return set(yaml.load(open(active_dcs, encoding="utf8"), Loader=yaml.FullLoader))

    def validate_mercury_hosts_ini(self):
        """Ensure the mercury hosts.ini file is properly formatted."""
        phy_nodes = set(self.get_deployment_physical_hosts())
        for node in self.serial_numbers.keys():
            if node not in phy_nodes:
                print(node)


if __name__ == "__main__":
    deployment = IcDeployment("mercury")
    deployment.validate_mercury_hosts_ini()
    # print(json.dumps(depl.get_deployment_nodes_hostvars(), indent=2))
