#!/usr/bin/env python3
"""
Proposal-submission script to set an upper limit of 120k canisters on subnets.

The script retrieves the list of subnets using the `ic-admin` command-line tool
and then checks the current canister limit for each subnet.
If the current limit is less than the desired limit, it sends a proposal using the `dre`
tool to propose an update to the subnet parameters to set the new limit.

The script requires the `ic-admin` and `dre` command-line tools to be installed
and configured.
No command line arguments are expected.
"""
import json
import subprocess


summary = """
## Summary and Motivation
DFINITY proposes to set a limit of 120,000 canisters per subnet, to avoid overloaded
subnets. More canisters on a subnet means more load on that subnet, and we have
measured that performance starts slowly degrading if subnet has more than 100,000 canisters.
By explicitly setting this limit in the subnet parameters, we expect more predictable
subnet performance.

The immediate impact on existing canisters is minimal, as the current highest number
of canisters on a subnet is approximately 80,000. If a subnet reaches this limit, new
canister creation attempts on that subnet will fail, but existing canisters won't be
affected. Canisters can still be created on other subnets.

## Forum Link
Please feel free to ask questions on the forum: https://forum.dfinity.org/t/proposal-to-set-canister-limit/28564
"""

MAX_CANISTERS_PER_SUBNET = 120_000

subnets = json.loads(
    subprocess.check_output("ic-admin --nns-urls https://ic0.app --json get-subnet-list".split()).decode("utf-8")
)

skip = True
for subnet_id in subnets[1:]:
    subnet_id_short = subnet_id.split("-")[0]
    if subnet_id_short == "3hhby":
        skip = False
    if skip:
        continue
    subnet_info = subprocess.check_output(
        "ic-admin --nns-urls https://ic0.app --json get-subnet".split() + [subnet_id]
    ).decode("utf-8")
    subnet_info = json.loads(subnet_info)
    curr_canister_limit = subnet_info["records"][0]["value"]["max_number_of_canisters"]
    if curr_canister_limit < MAX_CANISTERS_PER_SUBNET:
        subprocess.check_call(
            [
                "dre",
                "propose",
                "update-subnet",
                "--max-number-of-canisters",
                str(MAX_CANISTERS_PER_SUBNET),
                "--subnet",
                subnet_id,
                "--proposal-title",
                f"Setting an Upper Limit of 120k Canisters on subnet {subnet_id_short}",
                "--summary",
                summary,
            ]
        )
