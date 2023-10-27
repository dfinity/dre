#!/usr/bin/env python3
import argparse
import json
import logging
import pathlib
import shlex
import subprocess
import sys


def cmd(command_and_args, quiet=False, **kwargs):
    """Run a system command and return the output."""
    if not quiet:
        print("$", shlex.join(command_and_args))
    result = subprocess.check_output(command_and_args, **kwargs).decode("utf8").strip()
    if not quiet:
        print(result)
    return result


def parse_args():
    """Parse command line arguments."""

    class HelpfulParser(argparse.ArgumentParser):
        def error(self, message):
            sys.stderr.write("error: %s\n" % message)
            self.print_help()
            sys.exit(2)

    parser = HelpfulParser()

    parser.add_argument(
        "--deployment-name",
        action="store",
        default="staging",
        help="Deployment name (default: staging)",
    )

    parser.add_argument(
        "--type",
        action="store",
        choices=["add", "remove", "set", "update"],
        required=True,
        help="What type of firewall change is requested.",
    )

    parser.add_argument(
        "--position",
        action="store",
        required=True,
        help="Firewall position that needs to be changed, e.g. 0, 1, 2, ...",
    )

    parser.add_argument(
        "--motivation",
        action="store",
        required=True,
        help="The motivation behind changing the firewall rule.",
    )

    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose mode")

    args = parser.parse_args()

    if args.verbose:
        logging.basicConfig(level=logging.DEBUG)
    else:
        logging.basicConfig(level=logging.INFO)

    return args


def calculate_hash_of_new_rules(change_type, rules_file_path, rule_position):
    result = cmd(
        f"ic-admin --nns-url=https://ic0.app propose-to-{change_type}-firewall-rules --test replica_nodes".split()
        + [str(rules_file_path), rule_position, "none"],
        quiet=True,
    )
    result = json.loads(result)
    result = json.loads(result)
    print(json.dumps(result, indent=2))
    return result["hash"]


def submit_proposal(auth_options, change_type, rules_file_path, rule_position, new_rules_hash, motivation):
    result = cmd(
        ["release_cli"]
        + auth_options
        + [
            "propose",
            f"{change_type}-firewall-rules",
            "replica_nodes",
            str(rules_file_path),
            rule_position,
            new_rules_hash,
            "--summary",
            "Motivation: " + motivation,
        ],
        quiet=False,
    )
    return result


def main():
    args = parse_args()
    firewall_dir = pathlib.Path(__file__).parent
    rules_file_path = firewall_dir / "rules" / (args.position + ".json")

    deployment_name = args.deployment_name.lower()

    if deployment_name == "staging":
        auth_options = [
            "--network=staging",
            "--private-key-pem=$HOME/.config/dfx/identity/bootstrap-super-leader/identity.pem",
            "--neuron-id=49",
        ]
    elif deployment_name == "mainnet":
        auth_options = []
    else:
        logging.error("Invalid deployment name provided: %s", deployment_name)
        sys.exit(1)

    new_rules_hash = calculate_hash_of_new_rules(
        change_type=args.type, rules_file_path=rules_file_path, rule_position=args.position
    )

    result = submit_proposal(
        auth_options,
        change_type=args.type,
        rules_file_path=rules_file_path,
        rule_position=args.position,
        new_rules_hash=new_rules_hash,
        motivation=args.motivation,
    )

    print(result)


if __name__ == "__main__":
    main()
