#!/usr/bin/env python3
import argparse
import json
import logging
import subprocess
import tempfile
from pathlib import Path

import ip6calc
import yaml

IPV6_PREFIX_KEY = "ipv6_prefix"
IPV6_SUBNET_KEY = "ipv6_subnet"
LOCATION_KEY = "location"
NODE_OPERATOR_KEY = "node_operator"
DATACENTER_KEY = "datacenter"
HOSTNAME_KEY = "hostname"
SYSTEM_SERIAL_KEY = "system_serial"
IPV6_KEY = "ipv6"
NAME_KEY = "name"

VALUE_UNKNOWN = "Unknown"

EXCLUDE_DATACENTERS_TESTNET = ["ch1", "zh1", "fr1", "sf1"]
EXCLUDE_DATACENTERS_WAITING_HANDOVER = ["ch3", "bc1", "ph1", "hu1", "st1"]
EXCLUDE_DATACENTERS = EXCLUDE_DATACENTERS_TESTNET + EXCLUDE_DATACENTERS_WAITING_HANDOVER
EXCLUDE_HOSTS_STAGING = [
    "an1-dll26",
    "an1-dll27",
    "an1-dll28",
    "br1-dll26",
    "br1-dll27",
    "br1-dll28",
    "br2-dll12",
    "br2-dll13",
    "br2-dll14",
    "ch2-dll26",
    "ch2-dll27",
    "ch2-dll28",
    "dl1-dll26",
    "dl1-dll27",
    "dl1-dll28",
    "ge1-dll26",
    "ge1-dll27",
    "ge1-dll28",
    "ge2-dll26",
    "ge2-dll27",
    "ge2-dll28",
    "jv1-dll26",
    "jv1-dll27",
    "jv1-dll28",
    "or1-dll26",
    "or1-dll27",
    "or1-dll28",
    "pl1-dll26",
    "pl1-dll27",
    "pl1-dll28",
    "sg1-dll26",
    "sg1-dll27",
    "sg1-dll28",
    "sg2-dll12",
    "sg2-dll13",
    "sg2-dll14",
    "sg3-dll26",
    "sg3-dll27",
    "sg3-dll28",
]
EXCLUDE_HOSTS = EXCLUDE_HOSTS_STAGING


def hosts(data, datacenters, deployment_name: str):
    for host, system_serial in data.items():
        hostname_short = host.split(".")[0]
        if hostname_short in EXCLUDE_HOSTS:
            continue

        # Guess which nodes will be used for participating in the network as compute nodes
        if "dll" not in hostname_short and "spm" not in hostname_short:
            continue

        datacenter_key = hostname_short.split("-")[0]
        if datacenter_key in EXCLUDE_DATACENTERS:
            continue

        if datacenter_key not in datacenters:
            logging.warning("Skipped node %s: no datacenter info", hostname_short)
            continue
        datacenter = datacenters[datacenter_key]["vars"]

        yield {
            NAME_KEY: hostname_short,
            DATACENTER_KEY: datacenter_key,
            SYSTEM_SERIAL_KEY: system_serial,
            IPV6_KEY: ip6calc.ipv6_address_calculate_slaac(
                datacenter[IPV6_PREFIX_KEY], datacenter[IPV6_SUBNET_KEY], system_serial, deployment_name, "1"
            ),
        }


def main():
    logging.basicConfig(level=logging.INFO)

    parser = argparse.ArgumentParser()

    parser.add_argument(
        "--deployment-name",
        action="store",
        help='Deployment name (e.g. "mercury")',
        default="mercury",
    )
    args = parser.parse_args()

    with tempfile.TemporaryDirectory() as tmpdirname:
        subprocess.run(
            [
                "git",
                "clone",
                "--depth=1",
                "--branch=main",
                "git@gitlab.com:dfinity-lab/core/release-cli.git",
                tmpdirname,
            ],
            check=True,
        )
        data_path = Path(tmpdirname) / "deployments/env"

        hosts_data = yaml.safe_load(open(data_path / "serial-numbers.yml", "r"))
        datacenters_data = yaml.safe_load(open(data_path / "shared-config.yml", "r"))["data_centers"]
        results_dir = Path(__file__).resolve().parent.joinpath("..").joinpath("data")
        with open(results_dir.joinpath("hosts.json"), "w+") as f:
            f.write(
                json.dumps(
                    list(hosts(hosts_data, datacenters_data, args.deployment_name)),
                    indent=2,
                    sort_keys=True,
                    ensure_ascii=False,
                )
            )


if __name__ == "__main__":
    main()
