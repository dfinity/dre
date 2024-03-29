#!/usr/bin/env python3
"""Run ssh commands on a list of nodes, and return the execution status and stdout/stderr."""
import functools
import gzip
import json
import logging
import os
import pathlib
import platform
import shlex
import stat
import subprocess
import typing
from multiprocessing import cpu_count
from multiprocessing import Pool

import git
import paramiko
import requests
import yaml
from tenacity import retry
from tenacity import retry_if_not_exception_type
from tenacity import stop_after_attempt
from tenacity import wait_exponential

if os.environ.get("BAZEL") != "true":
    repo_root = os.environ.get("GIT_ROOT")
    if not repo_root:
        git_repo = git.Repo(os.path.dirname(__file__), search_parent_directories=True)
        repo_root = git_repo.git.rev_parse("--show-toplevel")
    repo_root = pathlib.Path(repo_root)

PHY_HOST_USER = "dfnadmin"
NNS_URL = os.environ.get("NNS_URL") or "http://[2a00:fb01:400:100:5000:5bff:fe6b:75c6]:8080"


class IcSshRemoteRun:
    """Run commands remotely over ssh on the deployment nodes."""

    def __init__(self, deployment_name: str, out_dir: pathlib.Path, node_filter: str = "", physical_limit=None):
        """Create an object for the specified deployment and node filter, storing results in out_dir."""
        self._deployment_name = deployment_name
        deployment_env_root = find_deployment_env_root(deployment_name)
        self._inventory_script = deployment_env_root.parent.parent / "ansible/inventory/inventory.py"
        self._node_filter = node_filter
        # List of short physical hostnames to which the execution should be limited to
        # e.g. {'zh2-spm01'}
        self._phy_short_limit = set() if physical_limit is None else physical_limit
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
                self._inventory_script,
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
                self._inventory_script,
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
                self._inventory_script,
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
            run_results += parallel_ssh_run(nodes_external, username=PHY_HOST_USER, command=command_external)
        if nodes_dfinity:
            run_results += parallel_ssh_run(nodes_dfinity, username=PHY_HOST_USER, command=command_dfinity)
        for node, run_result in zip(nodes_external + nodes_dfinity, run_results):
            rc, stdout, stderr = run_result
            if rc:
                logging.error("ERROR (%s) at node %s", rc, node)
                logging.error("STDOUT: %s", stdout)
                logging.error("STDERR: %s", stderr)
                raise RuntimeError("Node %s Step failed (rc=%s)\nSTDOUT: %s\nSTDERR: %s" % (node, rc, stdout, stderr))
            else:
                logging.debug("Node %s OK STDOUT: %s", node, stdout)

    def run_on_guests(self, nodes: typing.List, username: str, command: str):
        """Run the command on the nodes list and return the result."""
        return parallel_ssh_run_without_raise(list(nodes), username, command)

    def check_run_on_guests(self, nodes: typing.List, username: str, command: str):
        """Run the command on the nodes list and return the result, ensuring command success."""
        return parallel_ssh_run(list(nodes), username=username, command=command)


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
            self._physical_hosts_limit = []
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
            "--skip-tags=boundary_node_vm",
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


def find_deployment_env_root(deployment_name: str):
    """Search for the deployment in the "deployments" subdirectory and fall back to "ic/testnet"."""
    deployment_env_root = repo_root / "deployments/env" / deployment_name
    if deployment_env_root.exists():
        return deployment_env_root
    testnet_env_root = repo_root / "ic/testnet/env" / deployment_name
    if testnet_env_root.exists():
        return testnet_env_root
    raise ValueError(
        "Deployment %s not found. Looked at %s and %s" % (deployment_name, deployment_env_root, testnet_env_root)
    )


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


@retry(
    reraise=True,
    stop=stop_after_attempt(5),
    wait=wait_exponential(multiplier=1, min=2, max=10),
)
def download_ic_binary(remote_path: str, blessed: bool = True):
    """Download an IC binary from from the DFINITY CDN and return the binary bytes."""
    if blessed:
        url = f"https://download.dfinity.systems/blessed/ic/{remote_path}"
    else:
        url = f"https://download.dfinity.systems/ic/{remote_path}"
    logging.info("Downloading: %s", url)
    resp = requests.get(url, timeout=30)
    resp.raise_for_status()
    return resp.content


def compute_ic_executable_path(executable_name: str, git_revision: str):
    """Return the local canister path for the given canister name and git revision."""
    return pathlib.Path.home() / "bin" / f"{executable_name}.{git_revision}"


def download_ic_executable(git_revision: str, executable_name: str, blessed: bool = False):
    """Download a platform-specific executable for the given git revision and return the local path."""
    local_path = compute_ic_executable_path(executable_name=executable_name, git_revision=git_revision)
    if local_path.exists() and local_path.stat().st_size > 0 and os.access(local_path, os.X_OK):
        logging.debug("Target file already exists: %s", local_path)
        return local_path

    platform_lower = platform.system().lower()
    remote_path = f"{git_revision}/binaries/x86_64-{platform_lower}/{executable_name}.gz"
    contents = download_ic_binary(remote_path=remote_path, blessed=blessed)

    local_path.parent.mkdir(exist_ok=True, parents=True)  # Ensure the parent directory exists
    with open(local_path, "wb") as f:
        f.write(gzip.decompress(contents))

    # Ensure that the file is marked as executable
    st = os.stat(local_path)
    os.chmod(local_path, st.st_mode | stat.S_IEXEC)
    return local_path


def compute_local_canister_path(canister_name: str, git_revision: str):
    """Return the local canister path for the given canister name and git revision."""
    return pathlib.Path.home() / "tmp" / "canisters" / f"{canister_name}.{git_revision}.wasm"


def download_ic_canister(git_revision: str, canister_name: str, blessed: bool = False):
    """Download a platform-specific executable for the given git revision and return the local path."""
    local_path = compute_local_canister_path(canister_name=canister_name, git_revision=git_revision)
    if local_path.exists() and local_path.stat().st_size > 0:
        logging.debug("Target file already exists: %s", local_path)
        return local_path

    remote_path = f"{git_revision}/canisters/{canister_name}.wasm.gz"
    contents = download_ic_binary(remote_path=remote_path, blessed=blessed)

    local_path.parent.mkdir(exist_ok=True, parents=True)  # Ensure the parent directory exists
    with open(local_path, "wb") as f:
        f.write(gzip.decompress(contents))
    return local_path
