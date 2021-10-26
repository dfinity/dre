import json
import logging
import os
import pathlib
import shlex
import subprocess
import typing
from multiprocessing import Pool

import git
import paramiko
import yaml

git_repo = git.Repo(os.path.dirname(__file__), search_parent_directories=True)
repo_root = pathlib.Path(git_repo.git.rev_parse("--show-toplevel"))
PHY_HOST_USER = "dfnadmin"
NNS_URL = os.environ.get("NNS_URL") or "http://[2001:470:1:c76:5000:2cff:fe0c:f490]:8080"


class IcAdmin:
    """Interface with the ic-admin utility."""

    def __init__(self, ic_admin_path: pathlib.Path = "ic-admin", nns_url: str = NNS_URL):
        """Create an object with the specified ic-admin path and NNS URL."""
        self.ic_admin_path = ic_admin_path
        self.nns_url = nns_url

    def _ic_admin_run(self, *cmd):
        return subprocess.check_output([self.ic_admin_path, "--nns-url", self.nns_url, *cmd])

    def get_nns_public_key(self, out_filename):
        """Save the NNS public key in the specified out_filename."""
        if not os.path.exists(out_filename):
            self._ic_admin_run("get-subnet-public-key", "0", out_filename)


class IcSshRemoteRun:
    """Run commands remotely over ssh on the deployment nodes."""

    def __init__(self, deployment_name: str, out_dir: pathlib.Path, node_filter: str = None):
        """Create an object for the specified deployment and node filter, storing results in out_dir."""
        self._deployment_name = deployment_name
        self._node_filter = node_filter
        self._out_dir = out_dir

        out_dir.mkdir(exist_ok=True, parents=True)
        paramiko.util.log_to_file(out_dir / "paramiko.log", level="WARN")

    def get_deployment_list(self):
        """Get the JSON representation of the Ansible deployment (same as 'hosts --list')."""
        env = {"PATH": os.environ.get("PATH", "/usr/local/bin:/usr/bin:/bin")}
        if self._node_filter:
            env["NODES_FILTER_INCLUDE"] = self._node_filter
        output = subprocess.check_output(
            [
                repo_root / "ic/testnet/ansible/inventory/inventory.py",
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
            if json_list["_meta"]["hostvars"][phy_fqdn].get("external"):
                nodes_external.append(phy_fqdn)
            else:
                nodes_dfinity.append(phy_fqdn)
        return nodes_external, nodes_dfinity

    def _deployment_nodes_dict(self):
        # First prepare a list of all nodes in the deployment
        env = {"PATH": os.environ.get("PATH", "/usr/local/bin:/usr/bin")}
        if self._node_filter:
            env["NODES_FILTER_INCLUDE"] = self._node_filter

        output = subprocess.check_output(
            [
                repo_root / "ic/testnet/ansible/inventory/inventory.py",
                "--deployment",
                self._deployment_name,
                "--nodes",
            ],
            env=env,
        )
        return yaml.load(output, Loader=yaml.FullLoader)

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

    def _ssh_run_command(self, node: typing.List, command: str, username: str, do_not_raise: bool = False):
        """SSH into a node, run the command, and return (exit_code, stdout, stderr)."""
        client = paramiko.SSHClient()
        client.set_missing_host_key_policy(paramiko.AutoAddPolicy())

        logging.info("ssh %s@%s %s", username, node, command)

        try:
            client.connect(
                node, port=22, username=username, timeout=10, auth_timeout=30, allow_agent=True, look_for_keys=True
            )
        except paramiko.ssh_exception.AuthenticationException as e:
            logging.error("Auth failed for %s@%s %s", username, node, e)
            if do_not_raise:
                return -12, b"", b"Auth failed: " + str(e).encode("utf-8")
            else:
                raise
        except Exception as e:  # noqa
            logging.error("Error connecting to %s@%s %s", username, node, e)
            if do_not_raise:
                return -10, b"", b"Error connecting: " + str(e).encode("utf-8")
            else:
                raise

        (_stdin, stdout, stderr) = client.exec_command(f"timeout 10 bash -c {shlex.quote(command)}")
        stdout.channel.settimeout(10)
        stderr.channel.settimeout(10)
        return stdout.channel.recv_exit_status(), stdout.read(), stderr.read()

    def _parallel_ssh_run(self, nodes: typing.List[str], command: str, username: str):
        """Parallel ssh into the `nodes`, run `command`."""
        with Pool(16) as pool:
            return pool.starmap(
                self._ssh_run_command,
                map(lambda n: (n, command, username), nodes),
            )

    def _parallel_ssh_run_without_raise(self, nodes: typing.List[str], command: str, username: str):
        """Parallel ssh into the `nodes`, run `command` and return results, without raising an exception."""
        with Pool(16) as pool:
            return pool.starmap(
                self._ssh_run_command,
                map(lambda n: (n, command, username, True), nodes),
            )

    def check_run_on_physical_nodes(self, command_external: str, command_dfinity: str):
        """Run the command_external on non-dfinity nodes and command_dfinity on DFINITY-owned nodes."""
        nodes_external, nodes_dfinity = self.get_physical_hosts()

        run_results = []
        if nodes_external:
            run_results += self._parallel_ssh_run(nodes_external, command_external, PHY_HOST_USER)
        if nodes_dfinity:
            run_results += self._parallel_ssh_run(nodes_dfinity, command_dfinity, PHY_HOST_USER)
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

    def run_on_guests(self, nodes: typing.List, command: str, username: str):
        """Run the command on the nodes list and return the result."""
        return self._parallel_ssh_run_without_raise(list(nodes), command, username)

    def check_run_on_guests(self, nodes: typing.List, command: str, username: str):
        """Run the command on the nodes list and return the result, ensuring command success."""
        return self._parallel_ssh_run(list(nodes), command, username)


class IcAnsible:
    """Run ansible playbooks or plain commands on the deployment nodes."""

    def __init__(self, deployment_name: str, node_filter: str = None):
        """Create an object for the specified deployment and node filter."""
        self._deployment_name = deployment_name
        self._node_filter = node_filter or ""
        self._hosts_file = repo_root / "testnet/env" / self._deployment_name / "hosts"

    def ansible_run_shell_checked(self, command: str):
        """Run the specified command on the deployment nodes and check that the execution succeeds."""
        env = {"NODES_FILTER_INCLUDE": self._node_filter, "PATH": os.environ.get("PATH", "/usr/local/bin:/usr/bin")}
        if os.environ.get("SSH_AUTH_SOCK"):
            env["SSH_AUTH_SOCK"] = os.environ.get("SSH_AUTH_SOCK")
        cmd = [
            "ansible",
            "-i",
            self._hosts_file.as_posix(),
            "physical_hosts",
            "--module-name",
            "shell",
            "--args",
            command,
            "--extra-vars",
            "ansible_user={}".format(PHY_HOST_USER),
        ]
        logging.info("$ %s", cmd)
        subprocess.check_call(cmd, env=env, cwd=repo_root / "testnet")

    def ansible_ic_guest_playbook_run(self, ic_state: str, extra_vars: typing.List[str] = []):
        """Run the ic_guest_prod playbook with the specified ic_state and check that the execution succeeds."""
        env = {"NODES_FILTER_INCLUDE": self._node_filter, "PATH": os.environ.get("PATH", "/usr/local/bin:/usr/bin")}
        if os.environ.get("SSH_AUTH_SOCK"):
            env["SSH_AUTH_SOCK"] = os.environ.get("SSH_AUTH_SOCK")
        cmd = [
            "ansible-playbook",
            "ansible/roles/ic_guest_prod/playbook.yml",
            "-i",
            self._hosts_file.as_posix(),
            "--limit",
            "physical_hosts",
            "--extra-vars",
            "ic_state=%s" % ic_state,
            "--extra-vars",
            "ansible_user={}".format(PHY_HOST_USER),
        ]
        if extra_vars:
            for entry in extra_vars:
                cmd.append("--extra-vars")
                cmd.append(entry)
        logging.info("$ NODES_FILTER_INCLUDE=%s %s", self._node_filter, " ".join([shlex.quote(_) for _ in cmd]))
        subprocess.check_call(cmd, env=env, cwd=repo_root / "testnet")
        logging.info("Ansible playbook finished successfully")
