#!/usr/bin/env python3
"""Interface with the deployment Ansible configuration, e.g. get nodes, ipv6 addresses, etc."""
import json
import os
import pathlib
import subprocess

import yaml


class IcDeployment:
    """Interface with the deployment Ansible configuration, e.g. get nodes, ipv6 addresses, etc."""

    def __init__(self, repo_root: pathlib.Path, deployment_name: str):
        """Create an object for the given git repo root and deployment name."""
        self.repo_root = repo_root
        self._name = deployment_name

    @property
    def name(self):
        """Return the deployment name."""
        return self._name

    def get_ansible_inventory(self, all_physical_hosts=False):
        """Get an Ansible inventory list for a deployment."""
        env = os.environ.copy()
        if all_physical_hosts:
            env["INCLUDE_ALL_PHYSICAL_HOSTS"] = "1"
        output = subprocess.check_output(
            [
                "ansible-inventory",
                "-i",
                self.repo_root / f"deployments/env/{self.name}/hosts",
                "--list",
            ],
            env=env,
        )
        return json.loads(output)

    def get_deployment_nodes_ipv6(self, nodes_filter_include=None):
        """Get a list of nodes for a deployment, as a dictionary of {node_name: ipv6}."""
        env = os.environ.copy()
        if nodes_filter_include:
            env["NODES_FILTER_INCLUDE"] = nodes_filter_include
        output = subprocess.check_output(
            [
                self.repo_root / "deployments/ansible/inventory/inventory.py",
                "--deployment",
                self.name,
                "--nodes",
            ],
            env=env,
        )
        return yaml.load(output, Loader=yaml.FullLoader)

    def get_deployment_subnet_nodes(self, subnet_name):
        """Get a list of nodes for a deployment, as a dictionary of {node_name: ipv6}."""
        output = subprocess.check_output(
            [
                self.repo_root / "deployments/ansible/inventory/inventory.py",
                "--deployment",
                self.name,
                "--list",
            ]
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
        serial_numbers_filename = self.repo_root / "deployments/env/serial-numbers.yml"
        all_serials = yaml.load(open(serial_numbers_filename, encoding="utf8"), Loader=yaml.FullLoader)
        return {k: v for k, v in all_serials.items() if k.split("-")[0] in self.mercury_dcs}

    @property
    def mercury_dcs(self):
        """Return a list of Mercury DCs."""
        active_dcs = self.repo_root / "factsdb/mercury-dcs.yml"
        return set(yaml.load(open(active_dcs, encoding="utf8"), Loader=yaml.FullLoader))

    def validate_mercury_hosts_ini(self):
        """Ensure the mercury hosts.ini file is properly formatted."""
        phy_nodes = set(self.get_deployment_physical_hosts())
        for node in self.serial_numbers.keys():
            if node not in phy_nodes:
                print(node)


if __name__ == "__main__":
    import git

    git_repo = git.Repo(pathlib.Path(__file__).parent, search_parent_directories=True)
    repo_root = pathlib.Path(git_repo.git.rev_parse("--show-toplevel"))

    depl = IcDeployment(repo_root, "mercury")
    depl.validate_mercury_hosts_ini()
    # print(json.dumps(depl.get_deployment_nodes_hostvars(), indent=2))
