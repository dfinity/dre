#!/usr/bin/env python3
#
# This script manages the Ansible facts for a deployment.
#
import argparse
import logging
import os
import pathlib
import sys

import __fix_import_paths  # noqa # pylint: disable=unused-import
import git
import model
import tabulate
import publish

import pylib.ic_admin as ic_admin
from pylib.ic_deployment import IcDeployment

repo_root = os.environ.get("GIT_ROOT")
if not repo_root:
    git_repo = git.Repo(pathlib.Path(__file__).parent, search_parent_directories=True)
    repo_root = git_repo.git.rev_parse("--show-toplevel")
repo_root = pathlib.Path(repo_root)

TABLE_FMT = "pretty"  # https://pypi.org/project/tabulate/


def main():
    parser = argparse.ArgumentParser()

    parser.add_argument(
        "--deployment-name",
        action="store",
        help="The deployment or testnet name.",
        default="mercury",
    )

    parser.add_argument(
        "--nns-url",
        action="store",
        help="The NNS node URL. Defaults to the first NNS node in the deployment.",
    )

    parser.add_argument(
        "--get-nns-url",
        action="store_true",
        help="Return the NNS node URL by parsing the deployment inventory.",
    )

    parser.add_argument(
        "--physical",
        action="store_true",
        help="File containing new line separated list of serial numbers",
    )

    parser.add_argument(
        "--update-serial-numbers",
        action="store_true",
        help="Update the serial numbers for the physical machines.",
    )

    parser.add_argument(
        "--update-hsm-public-keys",
        action="store_true",
        help="Update the HSM public keys on the physical machines for the deployment.",
    )

    parser.add_argument(
        "--principals",
        action="store_true",
        help="Work with Node principals",
    )

    parser.add_argument(
        "--principal",
        action="store",
        nargs="+",
        help="Return the principal for the host",
    )

    parser.add_argument(
        "--guests",
        action="store_true",
        help="Work with Nodes (guests)",
    )

    parser.add_argument(
        "--dump-filter",
        action="store",
        help="Regex filter to apply before dumping entries, e.g. --dump-filter subnet='kvdry-.+'",
    )

    parser.add_argument(
        "--dump-fields",
        action="store",
        nargs="*",
        default=[],
        help="A subset of fields to print if dumping",
    )

    parser.add_argument(
        "--subnets",
        action="store_true",
        help="Work with Subnets",
    )

    parser.add_argument(
        "--subnet-replica-revisions",
        action="store_true",
        help="A table of subnets and their replica versions",
    )

    parser.add_argument(
        "--filter-name",
        action="store",
        help="Entry name by which to filter.",
    )

    parser.add_argument(
        "--add",
        action="store",
        nargs="+",
        help="An entry of type 'name key1=value1 key2=value2' to add into the DB.",
    )

    parser.add_argument(
        "--set-values",
        action="store",
        nargs="+",
        help="A list of 'key=value' argument to set in the DB.",
    )

    parser.add_argument(
        "--refresh",
        action="store_true",
        help="Attempt to query missing data in the DB by calling ic-admin.",
    )

    parser.add_argument(
        "--refresh-hsm-public-keys",
        action="store_true",
        help="Refresh the HSM public keys by ssh-ing into the nodes.",
    )

    parser.add_argument(
        "--ansible-inventory",
        action="store_true",
        help="Check the Ansible inventory for discrepancies in subnets and membership.",
    )

    parser.add_argument(
        "-o",
        "--out",
        help="Where to write the output file (default is stdout)",
        type=argparse.FileType("w"),
        nargs="?",
        const="-",
        default="-",
    )

    parser.add_argument(
        "--publish-guests",
        action="store_true",
        help="Publish the guests file for the deployment.",
    )

    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose mode")

    args = parser.parse_args()

    if args.verbose:
        logging.basicConfig(level=logging.DEBUG)
    else:
        logging.basicConfig(level=logging.INFO)

    deployment = IcDeployment(
        args.deployment_name,
        nodes_filter_include=os.environ.get("NODES_FILTER_INCLUDE", ""),
        nns_url_override=args.nns_url,
    )
    model.deployment = deployment
    model.ic_adm = ic_admin.IcAdmin(deployment=deployment, git_revision=os.environ.get("IC_ADMIN_GIT_REVISION", None))
    file_storage = model.FileStorage()

    if args.subnet_replica_revisions:
        table = []
        headers = ["Subnet Id", "Revision"]
        for name, version in model.ic_adm.get_subnet_replica_versions().items():
            table.append([name, version])
        print(tabulate.tabulate(table, headers, tablefmt=TABLE_FMT, showindex="always"))
        sys.exit(0)

    model.db_open_and_load(file_storage)

    if args.ansible_inventory:
        model.Guest.generate_ansible_inventory()
        sys.exit(0)

    if args.update_serial_numbers:
        model.PhysicalSystem.update_repo_serial_numbers_yml()
        model.db_save_and_close(file_storage)
        sys.exit(0)

    if args.update_hsm_public_keys:
        model.PhysicalSystem.update_repo_hsm_public_keys()
        model.db_save_and_close(file_storage)
        sys.exit(0)

    if args.get_nns_url:
        print(deployment.get_nns_url())
        sys.exit(0)

    if args.refresh:
        model.PhysicalSystem.clear()
        model.PhysicalSystem.refresh(args.refresh_hsm_public_keys)
        if args.update_hsm_public_keys:
            # Don't update the subnet/principal/guest info if we update the HSM info
            # We update the HSM info at bootstrap and then there is no NNS Registry to query for the other info
            model.PhysicalSystem.update_repo_hsm_public_keys()
            model.PhysicalSystem.update_repo_serial_numbers_yml()
            model.db_save_and_close(file_storage)
            sys.exit(0)
        model.Subnet.clear()
        model.Subnet.refresh()
        model.Principal.clear()
        model.Principal.refresh()
        model.Guest.clear()
        model.Guest.refresh()

    if args.principal:
        for needle in args.principal:
            print(model.Guest.get_principal_for_physical_system(needle))
        sys.exit(0)

    if args.physical:
        table = model.PhysicalSystem
    elif args.principals:
        table = model.Principal
    elif args.guests:
        table = model.Guest
    elif args.subnets:
        table = model.Subnet
    elif args.refresh:
        # No command provided but a refresh was requested
        model.db_save_and_close(file_storage)
        sys.exit(0)
    elif args.publish_guests:
        model.db_save_and_close(file_storage)
        data_path = pathlib.Path(__file__).parent / "data" / (args.deployment_name + "_guests.csv")
        contents = data_path.read_text()
        publish.publish_data(filename=data_path.name, contents=contents)
        sys.exit(0)
    else:
        logging.error("You need to provide --physical, --guests, --principals, --subnets, or --publish-guests")
        parser.print_help()
        sys.exit(1)

    if args.set_values:
        update_args = {}
        for arg in args.set_values:
            k, v = arg.split("=", 1)
            update_args[k] = v
        table.set_values(name=args.filter_name, **update_args)

    model.dump_fields = args.dump_fields
    model.dump_filter = args.dump_filter
    table.dump(args.out)

    model.db_save_and_close(file_storage)


if __name__ == "__main__":
    main()
