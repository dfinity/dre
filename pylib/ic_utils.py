#!/usr/bin/env python3
"""Run ssh commands on a list of nodes, and return the execution status and stdout/stderr."""
import functools
import json
import logging
import os
import pathlib
import shlex
import subprocess
import typing
from multiprocessing import cpu_count
from multiprocessing import Pool

import git
import paramiko
import yaml
from tenacity import retry
from tenacity import retry_if_not_exception_type
from tenacity import stop_after_attempt
from tenacity import wait_exponential

git_repo = git.Repo(os.path.dirname(__file__), search_parent_directories=True)
repo_root = pathlib.Path(git_repo.git.rev_parse("--show-toplevel"))
PHY_HOST_USER = "dfnadmin"
NNS_URL = os.environ.get("NNS_URL") or "http://[2a00:fb01:400:100:5000:5bff:fe6b:75c6]:8080"


class IcSshRemoteRun:
    """Run commands remotely over ssh on the deployment nodes."""

    def __init__(self, deployment_name: str, out_dir: pathlib.Path, node_filter: str = None, physical_limit=None):
        """Create an object for the specified deployment and node filter, storing results in out_dir."""
        self._deployment_name = deployment_name
        self._node_filter = node_filter
        # List of short physical hostnames to which the execution should be limited to
        # e.g. {'zh2-spm01'}
        self._phy_short_limit = physical_limit
        self._out_dir = out_dir

        out_dir.mkdir(exist_ok=True, parents=True)
        paramiko.util.log_to_file(out_dir / "paramiko.log", level="WARN")

    @functools.lru_cache(maxsize=32)
    def get_deployment_list(self):
        """Get the JSON representation of the Ansible deployment (same as 'hosts --list')."""
        env = {"PATH": os.environ.get("PATH", "/usr/local/bin:/usr/bin:/bin")}
        if self._node_filter:
            env["NODES_FILTER_INCLUDE"] = self._node_filter
        output = subprocess.check_output(
            [
                repo_root / "deployments/ansible/inventory/inventory.py",
                "--deployment",
                self._deployment_name,
                "--list",
            ],
            env=env,
        )
        return json.loads(output)

    def get_physical_hosts(self):
        """Get a list of physical hosts for a deployment, split into external and dfinity-owned."""
        json_list = self.get_deployment_list()

        nodes_external, nodes_dfinity = [], []
        for phy_fqdn in json_list["physical_hosts"]["hosts"]:
            phy_short = phy_fqdn.split(".")[0]
            if self._phy_short_limit and phy_short not in self._phy_short_limit:
                continue
            if json_list["_meta"]["hostvars"][phy_fqdn].get("external"):
                nodes_external.append(phy_fqdn)
            else:
                nodes_dfinity.append(phy_fqdn)
        return nodes_external, nodes_dfinity

    @functools.lru_cache(maxsize=32)
    def _deployment_nodes_dict(self):
        # First prepare a list of all nodes in the deployment
        env = {"PATH": os.environ.get("PATH", "/usr/local/bin:/usr/bin")}
        if self._node_filter:
            env["NODES_FILTER_INCLUDE"] = self._node_filter

        output = subprocess.check_output(
            [
                repo_root / "deployments/ansible/inventory/inventory.py",
                "--deployment",
                self._deployment_name,
                "--nodes",
            ],
            env=env,
        )
        nodes_dict = yaml.load(output, Loader=yaml.FullLoader)

        if self._phy_short_limit:
            # Get a complete deployment inventory and filter by the short
            # physical hostnames provided in self._phy_short_limit
            json_list = self.get_deployment_list()
            result = {}
            for node in nodes_dict.keys():
                node_host_short = json_list["_meta"]["hostvars"][node].get("ic_host", "")
                if node_host_short in self._phy_short_limit:
                    result[node] = nodes_dict[node]
            return result

        return nodes_dict

    def get_deployment_nodes(self):
        """Get a list of nodes for a deployment, split into external and dfinity-owned."""
        nodes_dict = self._deployment_nodes_dict()
        # Now split the list of all nodes into the external and the dfinity-owned nodes
        json_list = self.get_deployment_list()

        nodes_external, nodes_dfinity = [], []
        for node in nodes_dict.keys():
            if "external" in json_list["_meta"]["hostvars"][node].get("node_type", "").split(","):
                nodes_external.append(node)
            else:
                nodes_dfinity.append(node)
        return nodes_external, nodes_dfinity

    def get_deployment_nodes_ipv6(self):
        """Get the ipv6 addresses for nodes in a deployment, split into external and dfinity-owned."""
        nodes_dict = self._deployment_nodes_dict()

        # Now split the list of all nodes into the external and the dfinity-owned nodes
        json_list = self.get_deployment_list()

        nodes_ipv6_external, nodes_ipv6_dfinity = [], []
        for node in nodes_dict.keys():
            node_ipv6 = json_list["_meta"]["hostvars"][node].get("ipv6")
            if "external" in json_list["_meta"]["hostvars"][node].get("node_type", "").split(","):
                nodes_ipv6_external.append((node, node_ipv6))
            else:
                nodes_ipv6_dfinity.append((node, node_ipv6))
        return nodes_ipv6_external, nodes_ipv6_dfinity

    def get_nns_urls(self):
        """Get the NNS nodes for the deployment, comma-separated."""
        env = {"PATH": os.environ.get("PATH", "/usr/local/bin:/usr/bin:/bin")}
        if self._node_filter:
            env["NODES_FILTER_INCLUDE"] = self._node_filter
        output = subprocess.check_output(
            [
                repo_root / "deployments/ansible/inventory/inventory.py",
                "--deployment",
                self._deployment_name,
                "--nns-nodes",
            ],
            env=env,
        )
        nns_nodes_dict = yaml.load(output, Loader=yaml.FullLoader)
        nns_nodes_urls = ["http://[%s]:8080" % ipv6 for ipv6 in nns_nodes_dict.values()]
        return ",".join(nns_nodes_urls)

    def check_run_on_physical_nodes(self, command_external: str, command_dfinity: str):
        """Run the command_external on non-dfinity nodes and command_dfinity on DFINITY-owned nodes."""
        nodes_external, nodes_dfinity = self.get_physical_hosts()

        run_results = []
        if nodes_external:
            run_results += parallel_ssh_run(nodes_external, command_external, PHY_HOST_USER)
        if nodes_dfinity:
            run_results += parallel_ssh_run(nodes_dfinity, command_dfinity, PHY_HOST_USER)
        for node, run_result in zip(nodes_external + nodes_dfinity, run_results):
            rc, stdout, stderr = run_result
            if rc:
                logging.error("ERROR (%s) at node %s", rc, node)
                logging.error("STDOUT: %s", stdout.decode("utf8"))
                logging.error("STDERR: %s", stderr.decode("utf8"))
                raise RuntimeError(
                    "Node %s Step failed (rc=%s)\nSTDOUT: %s\nSTDERR: %s"
                    % (node, rc, stdout.decode("utf"), stderr.decode("utf8"))
                )
            else:
                logging.debug("Node %s OK STDOUT: %s", node, stdout.decode("utf8"))

    def run_on_guests(self, nodes: typing.List, username: str, command: str):
        """Run the command on the nodes list and return the result."""
        return parallel_ssh_run_without_raise(list(nodes), username, command)

    def check_run_on_guests(self, nodes: typing.List, username: str, command: str):
        """Run the command on the nodes list and return the result, ensuring command success."""
        return parallel_ssh_run(list(nodes), command, username)


class IcAnsible:
    """Run ansible playbooks or plain commands on the deployment nodes."""

    def __init__(self, deployment_name: str, node_filter: str = None, physical_limit=None):
        """Create an object for the specified deployment and node filter."""
        self._deployment_name = deployment_name
        self._node_filter = node_filter or ""
        self._phy_short_limit = physical_limit
        if physical_limit:
            self._physical_hosts_limit = [f"{x}.{x.split('-')[0]}.dfinity.network" for x in self._phy_short_limit]
        else:
            self._physical_hosts_limit = None
        self._hosts_file = repo_root / "deployments/env" / self._deployment_name / "hosts"

    def ansible_run_shell_checked(self, command: str):
        """Run the specified command on the deployment nodes and check that the execution succeeds."""
        env = {
            "NODES_FILTER_INCLUDE": self._node_filter,
            "PATH": os.environ.get("PATH", "/usr/local/bin:/usr/bin"),
            "ANSIBLE_FORCE_COLOR": "true",
        }
        if os.environ.get("SSH_AUTH_SOCK"):
            env["SSH_AUTH_SOCK"] = os.environ.get("SSH_AUTH_SOCK")
        cmd = [
            "ansible",
            "-i",
            self._hosts_file,
            "physical_hosts",
            "--module-name",
            "shell",
            "--args",
            command,
            "--extra-vars",
            "ansible_user={}".format(PHY_HOST_USER),
        ]
        if self._physical_hosts_limit:
            cmd.append("--limit")
            cmd.append(",".join(self._physical_hosts_limit))
        cmd = [str(a) for a in cmd]
        logging.info("$ %s", cmd)
        subprocess.check_call(cmd, env=env, cwd=repo_root / "ic/testnet")

    def ansible_ic_guest_playbook_run(self, ic_state: str, extra_vars: typing.Optional[typing.List[str]] = None):
        """Run the ic_guest_prod playbook with the specified ic_state and check that the execution succeeds."""
        env = {
            "NODES_FILTER_INCLUDE": self._node_filter,
            "PATH": os.environ.get("PATH", "/usr/local/bin:/usr/bin"),
            "ANSIBLE_FORCE_COLOR": "true",
        }
        if os.environ.get("SSH_AUTH_SOCK"):
            env["SSH_AUTH_SOCK"] = os.environ.get("SSH_AUTH_SOCK")
        cmd = [
            "ansible-playbook",
            "ansible/roles/ic_guest_prod/playbook.yml",
            "-i",
            self._hosts_file,
            "--limit",
            "physical_hosts",
            "--extra-vars",
            "ic_state=%s" % ic_state,
            "--extra-vars",
            "ansible_user={}".format(PHY_HOST_USER),
        ]
        if self._physical_hosts_limit:
            cmd.append("--limit")
            cmd.append(",".join(self._physical_hosts_limit))
        if extra_vars:
            for entry in extra_vars:
                cmd.append("--extra-vars")
                cmd.append(entry)
        cmd = [str(a) for a in cmd]
        logging.info("$ NODES_FILTER_INCLUDE=%s %s", self._node_filter, " ".join([shlex.quote(_) for _ in cmd]))
        subprocess.check_call(cmd, env=env, cwd=repo_root / "ic/testnet")
        logging.info("Ansible playbook finished successfully")


def ssh_run_command(node: str, username: str, command: str, do_not_raise: bool = False, binary_stdout: bool = False):
    """SSH into a node, run the command, and return (exit_code, stdout, stderr)."""
    client = paramiko.SSHClient()
    client.set_missing_host_key_policy(paramiko.AutoAddPolicy())

    logging.info("ssh %s@%s %s", username, node, command)

    @retry(
        retry=retry_if_not_exception_type(paramiko.AuthenticationException),
        reraise=True,
        stop=stop_after_attempt(5),
        wait=wait_exponential(multiplier=1, min=2, max=10),
    )
    def paramiko_connect():
        client.connect(
            node, port=22, username=username, timeout=10, auth_timeout=30, allow_agent=True, look_for_keys=True
        )

    try:
        paramiko_connect()
    except paramiko.AuthenticationException as exc:
        logging.error("Auth failed for %s@%s %s", username, node, exc)
        if do_not_raise:
            return -12, b"", b"Auth failed: " + str(exc).encode("utf-8")
        else:
            raise
    except Exception as exc:  # noqa # pylint: disable=broad-except
        logging.exception("Error connecting to %s@%s", username, node)
        if do_not_raise:
            return -10, b"", b"Error connecting: " + str(exc).encode("utf-8")
        else:
            raise

    (_stdin, stdout, stderr) = client.exec_command(f"timeout 10 bash -c {shlex.quote(command)}", timeout=10)
    return (
        stdout.channel.recv_exit_status(),
        stdout.read() if binary_stdout else stdout.read().decode("utf8").rstrip("\r\n"),
        stderr.read().decode("utf8").rstrip("\r\n"),
    )


def parallel_ssh_run(nodes: typing.List[str], username: str, command: str, binary_stdout: bool = False):
    """Parallel ssh into the `nodes` and run `command`, return (exit_code, stdout, stderr)."""
    with Pool(cpu_count()) as pool:
        return pool.starmap(
            ssh_run_command,
            map(lambda n: (n, username, command, False, binary_stdout), nodes),
        )


def parallel_ssh_run_without_raise(nodes: typing.List[str], username: str, command: str, binary_stdout: bool = False):
    """Parallel ssh into the `nodes`, run `command` and return results, without raising an exception."""
    with Pool(cpu_count()) as pool:
        return pool.starmap(
            ssh_run_command,
            map(lambda n: (n, username, command, True, binary_stdout), nodes),
        )
