#!/usr/bin/env python3
import argparse
import functools
import inspect
import logging
import os
import pathlib
import re
import sys
import time
import typing

import __fix_import_paths  # noqa # pylint: disable=unused-import
import git
import paramiko
import tabulate

from pylib import ic_utils

GUEST_USER_ADMIN = "admin"
GUEST_USER_READONLY = "readonly"
TABLE_FMT = "simple"  # https://pypi.org/project/tabulate/
git_repo = git.Repo(os.path.dirname(__file__), search_parent_directories=True)
deployment_name: typing.Optional[str] = None
node_filter: typing.Optional[str] = None
nodes_external: typing.Optional[typing.List[str]] = None
nodes_dfinity: typing.Optional[typing.List[str]] = None
out_dir = None

# Check:
# which nodes are owned by us (where we have admin access)
# which version of the disk image is running
# does the node have dirty data from a previous subnet


@functools.lru_cache(maxsize=32)
def git_commit_get_date(commit):
    """Return the date of the provided git commit."""
    ic_submodule = git_repo.submodule("ic").module()
    return time.gmtime(ic_submodule.commit(commit).committed_date)


def step_1_ssh_readonly_user():
    """Ensure the deployment nodes are in the expected good state."""
    commands = """true"""

    ssh_run = ic_utils.IcSshRemoteRun(deployment_name, out_dir, node_filter)

    input("Insert the Yubikey for the readonly user and press ENTER to continue")

    # We need to have readonly access on all nodes
    nodes_all = nodes_external + nodes_dfinity
    run_results = ssh_run.run_on_guests(map(lambda x: x[1], nodes_all), GUEST_USER_READONLY, commands)
    for node, node_result in zip(nodes_all, run_results):
        return_code = node_result[0]
        assert return_code == 0, f"Readonly login failed for node {node}"


def step_1_ssh_admin_user():
    """Ensure the deployment nodes are in the expected good state."""
    commands = """true"""

    ssh_run = ic_utils.IcSshRemoteRun(deployment_name, out_dir, node_filter)

    input("Insert the Yubikey for the admin user and press ENTER to continue")

    # We need to have admin access on DFINITY-owned nodes, but not on the external ones
    run_results = ssh_run.run_on_guests(map(lambda x: x[1], nodes_external), GUEST_USER_ADMIN, commands)
    for node, node_result in zip(nodes_external, run_results):
        return_code = node_result[0]
        assert return_code != 0, f"Admin login should not succeed for external node {node}"

    run_results = ssh_run.run_on_guests(map(lambda x: x[1], nodes_dfinity), GUEST_USER_ADMIN, commands)
    for node, node_result in zip(nodes_dfinity, run_results):
        return_code = node_result[0]
        assert return_code == 0, f"Admin login failed for DFINITY-owned node {node}"


def step_2_versions():
    """Ensure the deployment nodes are in the expected good state."""
    commands = """
        V=/opt/ic/share/version.txt
        if [ -r /opt/dfinity/version.txt ]; then V=/opt/dfinity/version.txt; fi
        cat $V
    """

    input("Insert the Yubikey for the readonly user and press ENTER to continue")

    ssh_run = ic_utils.IcSshRemoteRun(deployment_name, out_dir, node_filter)

    nodes = nodes_external + nodes_dfinity
    nodes_ipv6 = map(lambda x: x[1], nodes)
    run_results = ssh_run.run_on_guests(nodes_ipv6, GUEST_USER_READONLY, commands)
    headers = ["Node", "IPv6", "Exit code", "Git Revision", "Commit date", "Stderr"]
    table = []
    commit_minimal = "fa5aedf9209cdc19a8a9451b862852fe0493a798"
    commit_minimal_date = git_commit_get_date(commit_minimal)
    logging.info(
        "Minimum acceptable commit: %s date: %s", commit_minimal, time.strftime("%Y-%m-%d %H:%M", commit_minimal_date)
    )
    for node, node_result in zip(nodes, run_results):
        return_code = node_result[0]
        commit = node_result[1]
        commit_date_str = ""
        if commit:
            commit_date = git_commit_get_date(commit)
            commit_date_str = time.strftime("%Y-%m-%d %H:%M", commit_date)
            if commit_date < commit_minimal_date:
                commit_date_str += " TOO OLD!!!"
            else:
                commit_date_str += " OK"
        stderr = node_result[2]
        table.append(node + (return_code, commit, commit_date_str, stderr))
    print(tabulate.tabulate(table, headers, tablefmt=TABLE_FMT))


def step_3_no_state():
    """Ensure the deployment nodes do not have existing state."""
    commands = """
        if [ -z "$(ls -A /var/lib/ic/data/ic_state/)" ]; then
            echo "Empty"
        else
            echo "Non-empty IC state folder!"
            exit 1
        fi
    """

    input("Insert the Yubikey for the readonly user and press ENTER to continue")

    nodes = nodes_external + nodes_dfinity
    nodes_ipv6 = map(lambda x: x[1], nodes)

    ssh_run = ic_utils.IcSshRemoteRun(deployment_name, out_dir, node_filter)
    ssh_run.check_run_on_guests(nodes_ipv6, GUEST_USER_READONLY, commands)


def main():
    # pylint: disable=global-statement
    global deployment_name
    global node_filter
    global nodes_external
    global nodes_dfinity
    global out_dir

    class HelpfulParser(argparse.ArgumentParser):
        def error(self, message):
            sys.stderr.write("error: %s\n" % message)
            self.print_help()
            sys.exit(2)

    parser = HelpfulParser()

    parser.add_argument(
        "--deployment-name",
        action="store",
        default="mercury",
        help="Deployment name (default: mercury)",
    )

    parser.add_argument(
        "--node-filter",
        action="store",
        help="Filter for the deployment nodes, example: 'node_type=batch_1'",
    )

    parser.add_argument(
        "--nodes",
        action="store",
        nargs="+",
        help="A list of nodes to run on.",
    )

    parser.add_argument(
        "--list-steps",
        "--ls",
        "--dry-run",
        action="store_true",
        help="A list of steps.",
    )

    parser.add_argument(
        "--step-filter",
        action="store",
        nargs="+",
        default="step",
        help="A regular expression filter for the steps to run.",
    )

    parser.add_argument(
        "--out-dir",
        action="store",
        help="The directory where the debug information should be written.",
        default=pathlib.Path.home() / "tmp" / "mercury",
    )

    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose mode")

    args = parser.parse_args()

    if args.verbose:
        logging.basicConfig(level=logging.DEBUG)
    else:
        logging.basicConfig(level=logging.INFO)

    if len(sys.argv) <= 1:
        parser.print_help()
        sys.exit(0)

    deployment_name = args.deployment_name
    node_filter = args.node_filter

    out_dir = args.out_dir
    if isinstance(out_dir, str):
        out_dir = pathlib.Path(out_dir)
    out_dir.mkdir(exist_ok=True, parents=True)

    paramiko.util.log_to_file(out_dir / "paramiko.log", level="WARN")

    ssh_run = ic_utils.IcSshRemoteRun(deployment_name, out_dir, node_filter)
    if args.nodes:
        nodes_external, nodes_dfinity = ssh_run.get_deployment_nodes_ipv6()

        def _node_substring_in_users_nodes_list(_nodes_list):
            """Iterate over nodes and return the ones intersecting with the list provided on the command line."""
            result = []
            for node_haystack in _nodes_list:
                for node_needle in args.nodes:
                    if re.search(node_needle, node_haystack[0]):
                        result.append(node_haystack)
            return result

        nodes_external = _node_substring_in_users_nodes_list(nodes_external)
        nodes_dfinity = _node_substring_in_users_nodes_list(nodes_dfinity)
    else:
        nodes_external, nodes_dfinity = ssh_run.get_deployment_nodes_ipv6()

    all_steps = [
        obj
        for name, obj in inspect.getmembers(sys.modules[__name__])
        if (inspect.isfunction(obj) and name.startswith("step_") and obj.__module__ == __name__)
    ]

    if args.list_steps:
        table = []
        headers = ["Step name", "Description"]
        for step in all_steps:
            if any([re.search(_, step.__name__) for _ in args.step_filter]):
                table.append([step.__name__, step.__doc__ or ""])
        print(tabulate.tabulate(table, headers, tablefmt=TABLE_FMT))
        sys.exit(0)

    if args.step_filter:
        for step in all_steps:
            if any([re.search(_, step.__name__) for _ in args.step_filter]):
                logging.info("\n\n%s\nRunning step %s\n%s", "*" * 80, step.__name__, "*" * 80)
                start = time.time()
                step()
                end = time.time()
                logging.info("Success %s in %.2f seconds", step.__name__, end - start)
    else:
        parser.print_help()
        print("\n")
        logging.info("No step filter provided. List of all steps:")
        table = []
        headers = ["Step name", "Description"]
        for step in all_steps:
            table.append([step.__name__, step.__doc__ or ""])
        print(tabulate.tabulate(table, headers, tablefmt=TABLE_FMT))


if __name__ == "__main__":
    main()
