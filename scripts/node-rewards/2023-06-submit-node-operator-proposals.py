#!/usr/bin/env python3
import argparse
import csv
import json
import pathlib
import subprocess

# The cost of submitting a proposal
ICP_PER_PROPOSAL = 10
NEURON_ID = 4037
QUILL_BIN = pathlib.Path.home() / "bin" / "quill"
RELEASE_CLI_BIN = pathlib.Path.home() / "bin" / "release_cli"
IC_ADMIN_BIN = pathlib.Path.home() / "bin" / "ic-admin.86dca51820a5a3ba4fd2ce550b8e0f66d1a94422"


def parse_args():
    # parse command line arguments
    parser = argparse.ArgumentParser(description="Submit node operator proposals")
    parser.add_argument("--csv", type=str, required=True, help="CSV file containing node operator proposals")
    return parser.parse_args()


def read_csv_file(filename: str):
    # Read a CSV file and extract the node operators and their rewards
    with open(filename) as csvfile:
        return list(csv.DictReader(csvfile))


def submit_proposal_adjust_rewards(node_operator: str, **rewards):
    # Submit an NNS proposal for adjusting the rewards of a node operator
    print(f"Submitting proposal for {node_operator} to set rewards to: {rewards}")
    summary = (
        "Update the Node Provider rewards to include rewards for extra storage "
        "installed on the Gen1 node machines, in accordance with "
        "https://forum.dfinity.org/t/update-of-gen1-np-remuneration/10553/7 and "
        "https://dashboard.internetcomputer.org/proposal/122635"
    )
    title = "Update the Node Provider rewards for Node Operator principal " + node_operator.split("-")[0]
    subprocess.check_call(
        [
            RELEASE_CLI_BIN,
            "propose",
            "update-node-operator-config",
            "--node-operator-id",
            node_operator,
            "--rewardable-nodes",
            json.dumps(rewards),
            "--summary",
            summary,
            "--proposal-title",
            title,
        ]
    )


def how_many_proposals_can_neuron_submit(neuron_id):
    # Get the number of proposals that can be submitted by a neuron
    neuron_info = (
        subprocess.check_output([QUILL_BIN, "get-neuron-info", str(neuron_id), "--yes"]).decode("utf-8").strip()
    )
    # many lines, plus balance similar to: '      stake_e8s = 10_100_000_000 : nat64;'
    stake_e8s = [line for line in neuron_info.splitlines() if "stake_e8s" in line][0]
    stake_e8s = stake_e8s.strip().split("=")[1].split(":")[0].strip().replace("_", "")
    return int(int(stake_e8s) // 1e8 // ICP_PER_PROPOSAL)


def get_current_rewards(node_operator: str):
    # Get the current rewards for a node operator
    out = (
        subprocess.check_output(
            [IC_ADMIN_BIN, "--nns-url", "https://ic0.app", "get-node-operator", node_operator, "--json"]
        )
        .decode("utf-8")
        .strip()
    )
    return json.loads(out)["value"]["rewardable_nodes"]


if __name__ == "__main__":
    args = parse_args()
    rows = read_csv_file(args.csv)
    balance = how_many_proposals_can_neuron_submit(NEURON_ID)
    print(f"Neuron {NEURON_ID} can submit {balance} proposals")
    for row in rows:
        if not row:
            break
        print(row)
        new_rewards = {k: int(v) for k, v in row.items() if k.lower().startswith("type")}
        if balance == 0:
            print("No more proposals can be submitted")
            break
        current_rewards = get_current_rewards(row["node-operator-principal"])
        # Only update rewards if a new non-zero value is set,
        # or if an existing reward needs to be updated to a new value
        # For instance
        # current_rewards = {"type0": 28}
        # new_rewards = {"type0": 0, "type1": 28, "type2": 0}
        # ==> new_rewards["type2"] does not need to be set to zero
        # ==> new_rewards = {"type0": 0, "type1": 28}
        new_rewards = {k: v for k, v in new_rewards.items() if (v != 0) or (k in current_rewards)}
        if current_rewards == new_rewards:
            print("Rewards are already set to", new_rewards, " ... skipping")
        else:
            print("Rewards are currently set to: ", current_rewards)
            submit_proposal_adjust_rewards(row["node-operator-principal"], **new_rewards)
        balance -= 1
