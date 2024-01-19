#!/usr/bin/env python3
import argparse
import logging
import os
import tempfile
from pathlib import Path
from pprint import pprint

import requests
import yaml
from git import Repo
from git.remote import Remote

SCRIPT_NAME = "boundary-nodes-updater"


def get_logger():
    FORMAT = "[%(asctime)s] %(levelname)-8s %(message)s"
    logging.basicConfig(format=FORMAT, level=logging.INFO)
    return logging.getLogger(SCRIPT_NAME)


def parse():
    parser = argparse.ArgumentParser(description="Script to push boundary nodes to service discovery")
    parser.add_argument("sd_url", help="Service discovery url, i.e. http://localhost:8000")
    parser.add_argument("repo_key", help="Key to access the repo")

    return parser.parse_args()


def git_sparse_checkout(repo_url, dir_path, branch, checked_out):
    repo = Repo.init(dir_path)
    origin = Remote(repo, "origin")
    if not origin.exists():
        Remote.create(repo, "origin", repo_url)

    git = repo.git()
    git.remote()
    git.config("core.sparseCheckout", "true")
    git.config("pull.rebase", "true")

    sparse_checkout_path = Path(dir_path) / ".git/info/sparse-checkout"
    for p in checked_out:
        # Will overwrite everything in the file everytime
        with open(sparse_checkout_path, "w") as f:
            f.write(p)

    git.pull("--depth", "1", "origin", branch)


def git_checkout(repo_url, dir_path, branch):
    repo = Repo.init(dir_path)
    origin = Remote(repo, "origin")
    if not origin.exists():
        Remote.create(repo, "origin", repo_url)
    origin.pull(f"refs/heads/{branch}:refs/heads/origin")


def main():
    args = parse()
    logging = get_logger()
    # TMP_DIR = "/tmp/boundary-nodes-scraping-config/"
    TMP_DIR = tempfile.TemporaryDirectory()
    TMP_DIR_PATH = TMP_DIR.name
    RELEASE_REPO_DIR = Path(TMP_DIR_PATH) / "release"

    key = args.repo_key.strip()

    git_sparse_checkout(
        f"https://oauth2:{key}@gitlab.com/dfinity-lab/core/release.git",
        RELEASE_REPO_DIR,
        "main",
        ["deployments/boundary-nodes/env/"],
    )

    # envs = next(os.walk(RELEASE_REPO_DIR / "deployments" / "boundary-nodes" / "env"))[1]
    envs = ["prod"]
    raw_hosts = {}
    for env in envs:
        with open(RELEASE_REPO_DIR / f"deployments/boundary-nodes/env/{env}/hosts.yml") as f:
            raw_hosts[env] = yaml.safe_load(f.read())["boundary"]["hosts"]

    hosts = {}
    for env in envs:
        if env not in hosts:
            hosts[env] = []
        for ansible_id, host in raw_hosts[env].items():
            new_host = {**host, "ansible_id": ansible_id}
            new_host["guest_addr"] = host["ipv6_address"].split("/")[0]
            new_host["host_addr"] = host["host_ipv6_address"]
            new_host["id"] = ansible_id
            new_host["labels"] = {
                "ansible_host": host["ansible_host"],
                "ansible_id": ansible_id,
            }
            hosts[env].append(new_host)

    config = {}
    config["sources"] = {}
    config["transforms"] = {}
    for env in envs:
        for node in hosts[env]:
            name = "-".join(node["ansible_host"].replace(".", "-").split("-")[0:2])
            guest_addr = "[" + node["guest_addr"] + "]:9100"
            host_addr = "[" + node["host_addr"] + "]:9100"

            guest = {
                "name": f"{name}-guest",
                "ic_name": "ic",
                "targets": [guest_addr],
                "job_type": "node_exporter",
                "custom_labels": {
                    "ic_node": node["ic_host"],
                    "dc": node["ic_host"].split("-")[0],
                    "address": guest_addr,
                },
            }

            response = requests.post(args.sd_url, json=guest)

            if response.status_code == 400:
                logging.warning(f"GuestOS Node {name} already exists in service discovery: error {response.text}")
            elif response.status_code == 200:
                logging.info(f"GuestOS Node {name} added to service discovery")
            else:
                logging.error(f"Failed to add GuestOS Node {name} to service discovery: error {response.json()}")
                os.exit(1)

            host = {
                "name": f"{name}-host",
                "ic_name": "ic",
                "targets": [host_addr],
                "job_type": "host_node_exporter",
                "custom_labels": {
                    "ic_node": node["ic_host"],
                    "dc": node["ic_host"].split("-")[0],
                    "address": host_addr,
                },
            }

            response = requests.post(args.sd_url, json=host)

            if response.status_code == 400:
                logging.warning(f"HostOS Node {name} already exists in service discovery: error {response.text}")
            elif response.status_code == 200:
                logging.info(f"HostOS Node {name} added to service discovery")
            else:
                logging.error(f"Failed to add HostOS Node {name} to service discovery: error {response.json()}")

    TMP_DIR.cleanup()


if __name__ == "__main__":
    main()
