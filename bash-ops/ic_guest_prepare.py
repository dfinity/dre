#!/usr/bin/env python3
import argparse
import inspect
import logging
import os
import pathlib
import re
import shlex
import subprocess
import sys
import time

import git
import ic_utils
import tabulate

PHY_HOST_USER = "dfnadmin"
DIRNAME_SSH_KEYS_DFINITY_NODES = "ssh_keys_dfinity_nodes"
DIRNAME_SSH_KEYS_EXTERNAL_NODES = "ssh_keys_external_nodes"
NNS_URL = os.environ.get("NNS_URL") or "http://[2001:470:1:c76:5000:2cff:fe0c:f490]:8080"
TABLE_FMT = "simple"  # https://pypi.org/project/tabulate/
git_repo = git.Repo(os.path.dirname(__file__), search_parent_directories=True)
repo_root = pathlib.Path(git_repo.git.rev_parse("--show-toplevel"))
deployment_name: str = None
node_filter: str = None
git_revision = None
out_dir = None


def step_1_sanity_check_nodes():
    """Ensure the deployment nodes are in the expected good state."""
    commands_external_nodes = """
        set -eEou pipefail
        hostname
        ip -6 addr | grep global | grep inet6
        if virsh list --all --name | grep .; then exit 1; fi
        """
    commands_dfinity_nodes = commands_external_nodes + "lsusb | grep -E 'Clay|Nitro'"  # DFINITY nodes need an HSM

    ssh_run = ic_utils.IcSshRemoteRun(deployment_name, out_dir, node_filter)

    ssh_run.check_run_on_physical_nodes(commands_external_nodes, commands_dfinity_nodes)


def step_1_sanity_check_lockout():
    """Ensure the lockout service on the deployment nodes is in the expected state."""
    commands_external_nodes = """
    set -eEou pipefail
    systemctl is-active dfinity-lockdown
    """
    commands_dfinity_nodes = """
    set -eEou pipefail
    ! systemctl is-active dfinity-lockdown
    """

    ssh_run = ic_utils.IcSshRemoteRun(deployment_name, out_dir, node_filter)

    ssh_run.check_run_on_physical_nodes(commands_external_nodes, commands_dfinity_nodes)


def step_1_sanity_check_ssh_keys():
    """Ensure the ssh keys are valid."""
    depl_env_root = repo_root / "testnet/env" / deployment_name
    for dir in [depl_env_root / DIRNAME_SSH_KEYS_DFINITY_NODES, depl_env_root / DIRNAME_SSH_KEYS_EXTERNAL_NODES]:
        for file in ["admin", "backup", "readonly"]:
            path = dir / file
            if not path.exists:
                logging.error("SSH key file '%s' does not exist", path.absolute())
                sys.exit(1)
    path_external_nodes_admin_keys = depl_env_root / DIRNAME_SSH_KEYS_EXTERNAL_NODES / "admin"
    with open(path_external_nodes_admin_keys) as ssh_keys_file:
        for line in ssh_keys_file:
            if line.strip().startswith("#"):
                continue
            logging.error("External nodes may not have any SSH keys: '%s'", path_external_nodes_admin_keys.absolute())
            sys.exit(1)


def step_2_destroy_nodes():
    """Ensure that there is no existing deployment on the target machines."""
    ic_ansible = ic_utils.IcAnsible(deployment_name, node_filter)
    ic_ansible.ansible_ic_guest_playbook_run(ic_state="destroy")


def step_3_save_nns_public_key():
    """Save the NNS public key."""
    ic_admin = ic_utils.IcAdmin()

    ic_admin.get_nns_public_key(out_dir / "nns_public_key.pem")
    logging.info("Saved the NNS public key to %s", out_dir)


def generate_media_file(
    out_filename: pathlib.Path, path_ssh_keys: pathlib.Path, nns_public_key_filename: pathlib.Path, hostname: str
):
    cmd = [
        repo_root / "ic-os/guestos/scripts/build-bootstrap-config-image.sh",
        out_filename,
        "--accounts_ssh_authorized_keys",
        path_ssh_keys,
        "--nns_url",
        NNS_URL,
        "--nns_public_key",
        nns_public_key_filename,
        "--hostname",
        hostname,
    ]
    logging.info("$ %s", cmd)
    subprocess.check_call(cmd)


def step_4_generate_media_image():
    """Generate the media image for the new deployment."""
    ssh_run = ic_utils.IcSshRemoteRun(deployment_name, out_dir, node_filter)
    json_list = ssh_run.get_deployment_list()

    depl_env_root = repo_root / "testnet/env" / deployment_name
    nns_public_key_filename = out_dir / "nns_public_key.pem"
    media_path = out_dir / "media"
    media_path.mkdir(exist_ok=True, parents=True)

    nodes_external, nodes_dfinity = ssh_run.get_deployment_nodes()
    for node in nodes_external:
        hostname = json_list["_meta"]["hostvars"][node]["guest_hostname"]
        generate_media_file(
            out_filename=media_path / (node + ".img"),
            path_ssh_keys=depl_env_root / DIRNAME_SSH_KEYS_EXTERNAL_NODES,
            nns_public_key_filename=nns_public_key_filename,
            hostname=hostname,
        )
    for node in nodes_dfinity:
        hostname = json_list["_meta"]["hostvars"][node]["guest_hostname"]
        generate_media_file(
            out_filename=media_path / (node + ".img"),
            path_ssh_keys=depl_env_root / DIRNAME_SSH_KEYS_DFINITY_NODES,
            nns_public_key_filename=nns_public_key_filename,
            hostname=hostname,
        )


def step_5_create_guest_domains():
    """Create the guest domains for the deployment."""
    ic_ansible = ic_utils.IcAnsible(deployment_name, node_filter)
    media_path = out_dir / "media"
    if not git_revision:
        logging.error("This step requires the --git-revision argument.")
        sys.exit(1)
    extra_vars = [
        f"ic_git_revision={git_revision}",
        f"ic_media_path={shlex.quote(media_path.as_posix())}",
        f"ic_deployment_name={deployment_name}",
    ]
    ic_ansible.ansible_ic_guest_playbook_run(ic_state="create", extra_vars=extra_vars)


def step_6_start_dfinity_guest_domains():
    """Start the DFINITY-owned guest domains for the deployment."""
    node_filter_dfinity = node_filter + ",dfinity"
    ssh_run = ic_utils.IcSshRemoteRun(deployment_name, out_dir, node_filter_dfinity)
    _phy_external, phy_dfinity = ssh_run.get_physical_hosts()
    if phy_dfinity:
        ic_ansible = ic_utils.IcAnsible(deployment_name, node_filter_dfinity)
        ic_ansible.ansible_ic_guest_playbook_run(ic_state="start")
    else:
        logging.info("Skipping since there are no DFINITY-owned nodes in the selection.")


def main():
    global deployment_name
    global node_filter
    global git_revision
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
        default="node_type=batch_1",
        help="Filter for the deployment nodes, example: 'node_type=batch_1'",
    )

    parser.add_argument(
        "--git-revision",
        action="store",
        help="Git revision to deploy.",
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
    git_revision = args.git_revision

    out_dir = args.out_dir
    if isinstance(out_dir, str):
        out_dir = pathlib.Path(out_dir)
    out_dir.mkdir(exist_ok=True, parents=True)

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
