#!/usr/bin/env python3
"""Run ssh commands on a list of nodes, and return the execution status and stdout/stderr."""
import gzip
import logging
import os
import pathlib
import platform
import shlex
import stat
import typing
from multiprocessing import cpu_count
from multiprocessing import Pool

import git
import paramiko
import requests
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
