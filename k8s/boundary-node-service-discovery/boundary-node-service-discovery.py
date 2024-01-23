#!/usr/bin/env python3
import argparse
import base64
import logging
import os

import requests
import yaml

SCRIPT_NAME = "boundary-nodes-updater"


def get_logger():
    FORMAT = "[%(asctime)s] %(levelname)-8s %(message)s"
    logging.basicConfig(format=FORMAT, level=logging.INFO)
    return logging.getLogger(SCRIPT_NAME)


def parse():
    parser = argparse.ArgumentParser(description="Script to push boundary nodes to service discovery")
    parser.add_argument("sd_url", help="Service discovery url, i.e. http://localhost:8000")

    return parser.parse_args()


def main():
    args = parse()
    logging = get_logger()

    key = os.environ.get("RELEASE_REPO_KEY", "")
    if not key:
        logging.error("RELEASE_REPO_KEY environment variable not found.")
        exit(1)

    # envs = next(os.walk(RELEASE_REPO_DIR / "deployments" / "boundary-nodes" / "env"))[1]
    envs = ["prod"]
    raw_hosts = {}
    for env in envs:
        response = requests.get(
            f"https://gitlab.com/api/v4/projects/29756090/repository/files/deployments%2Fboundary-nodes%2Fenv%2F{env}%2Fhosts.yml?ref=main",
            headers={"PRIVATE-TOKEN": key},
        )
        if response.status_code != 200:
            logging.error("Couldn't fetch code due to error: %s", response.text)
            exit(1)
        content = base64.b64decode(response.json()["content"])
        raw_hosts[env] = yaml.safe_load(content.decode())["boundary"]["hosts"]

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
                logging.warning("GuestOS Node %s already exists in service discovery: error %s", name, response.text)
            elif response.status_code == 200:
                logging.info("GuestOS Node %s added to service discovery", name)
            else:
                logging.error("Failed to add GuestOS Node %s to service discovery: error %s", name, response.json())
                exit(1)

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
                logging.warning("HostOS Node %s already exists in service discovery: error %s", name, response.text)
            elif response.status_code == 200:
                logging.info("HostOS Node %s added to service discovery", name)
            else:
                logging.error("Failed to add HostOS Node %s to service discovery: error %s", name, response.json())


if __name__ == "__main__":
    main()
