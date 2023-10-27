#!/usr/bin/env python3
import argparse
import json
import logging
import os
import subprocess
import sys

import __fix_import_paths  # noqa # pylint: disable=unused-import

from pylib import ic_utils
from pylib.ic_admin import IcAdmin

# from pylib.ic_deployment import IcDeployment


saved_versions_local_path = "testnet/mainnet_revisions.json"
saved_versions_path = ic_utils.repo_root / "ic" / saved_versions_local_path
nns_subnet_id = "tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe"


def get_saved_versions():
    """
    Return a dict with all saved versions.

    Example of the file contents:
    {
        "subnets": {
            "tbd26...": "xxxxxREVISIONxxx"
        },
    }
    The file can also be extended with other data, e.g., canister versions:
    {
        "canisters" {
            "rwlgt-iiaaa-aaaaa-aaaaa-cai": "xxxxxREVISIONxxx"
        }
    }
    """
    if saved_versions_path.exists():
        with open(saved_versions_path, "r", encoding="utf-8") as f:
            return json.load(f)
    else:
        return {}


def update_saved_subnet_version(subnet: str, version: str):
    """Update the version that we last saw on a particular IC subnet."""
    saved_versions = get_saved_versions()
    subnet_versions = saved_versions.get("subnets", {})
    subnet_versions[subnet] = version
    saved_versions["subnets"] = subnet_versions
    with open(saved_versions_path, "w", encoding="utf-8") as f:
        json.dump(saved_versions, f, indent=2)


def get_saved_nns_subnet_version():
    """Get the last known version running on the NNS subnet."""
    saved_versions = get_saved_versions()
    return saved_versions.get("subnets", {}).get(nns_subnet_id, "")


def main():
    """Do the main work."""

    class HelpfulParser(argparse.ArgumentParser):
        """An argparse parser that prints usage on any error."""

        def error(self, message):
            sys.stderr.write("error: %s\n" % message)
            self.print_help()
            sys.exit(2)

    parser = HelpfulParser()

    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose mode")

    args = parser.parse_args()

    if args.verbose:
        logging.basicConfig(level=logging.DEBUG)
    else:
        logging.basicConfig(level=logging.INFO)

    # if len(sys.argv) <= 1:
    #     parser.print_help()
    #     sys.exit(0)

    repo_root = ic_utils.repo_root
    remote_url = (
        subprocess.run(
            ["git", "remote", "get-url", "public-ic"], cwd=repo_root / "ic", capture_output=True, check=False
        )
        .stdout.decode("utf8")
        .strip()
    )

    if not remote_url:
        logging.info("Remote public-ic not found, adding it")
        subprocess.run(["git", "remote", "remove", "public-ic"], cwd=repo_root / "ic", capture_output=True, check=False)
        if os.environ.get("CI"):
            ic_repo_push_token = os.environ.get("IC_CREATE_VERSIONS_MR_TOKEN", "")
            remote_url = (
                f"https://release-create-ic-versions-mr:{ic_repo_push_token}@gitlab.com/dfinity-lab/public/ic.git"
            )
            subprocess.call(["git", "remote", "add", "public-ic", remote_url], cwd=repo_root / "ic")
        else:
            subprocess.call(
                ["git", "remote", "add", "public-ic", "git@gitlab.com:dfinity-lab/public/ic.git"], cwd=repo_root / "ic"
            )
    subprocess.check_call(["git", "fetch", "public-ic", "--prune"], cwd=repo_root / "ic")
    subprocess.check_call(["git", "reset", "--hard", "public-ic/post-merge-tests-passed"], cwd=repo_root / "ic")
    subprocess.check_call(["git", "clean", "-ffdx"], cwd=repo_root / "ic")
    ic_rev = (
        subprocess.check_output(["git", "rev-parse", "public-ic/post-merge-tests-passed"], cwd=repo_root / "ic")
        .decode("utf8")
        .strip()
    )

    ic_admin = IcAdmin(git_revision=ic_rev)

    current_nns_version = ic_admin.get_subnet_replica_version(subnet_num=0)
    logging.info("Current NNS subnet revision: %s", current_nns_version)

    if subprocess.call(["git", "checkout", "ic-mainnet-revisions"], cwd=repo_root / "ic") == 0:
        # The branch already exists, update the existing MR
        logging.info("Found an already existing target branch")
    else:
        subprocess.check_call(["git", "checkout", "-B", "ic-mainnet-revisions"], cwd=repo_root / "ic")
    subprocess.check_call(["git", "reset", "--hard", "public-ic/master"], cwd=repo_root / "ic")

    update_saved_subnet_version(subnet=nns_subnet_id, version=current_nns_version)
    git_modified_files = subprocess.check_output(
        ["git", "ls-files", "--modified", "--others"], cwd=repo_root / "ic"
    ).decode("utf8")
    if saved_versions_local_path in git_modified_files:
        logging.info("Creating/updating a MR that updates the saved NNS subnet revision")
        subprocess.check_call(["git", "add", saved_versions_local_path], cwd=repo_root / "ic")
        subprocess.check_call(["git", "config", "--global", "user.email", "infra@dfinity.org"], cwd=repo_root / "ic")
        subprocess.check_call(["git", "config", "--global", "user.name", "CI Automation"], cwd=repo_root / "ic")
        subprocess.check_call(
            ["git", "commit", "-m", "Update Mainnet IC revisions file", saved_versions_local_path],
            cwd=repo_root / "ic",
        )
        subprocess.check_call(
            [
                "git",
                "push",
                "--force",
                "--set-upstream",
                "public-ic",
                "ic-mainnet-revisions",
                "-o",
                "merge_request.create",
                "-o",
                "merge_request.target=master",
            ],
            cwd=repo_root / "ic",
        )


if __name__ == "__main__":
    main()
