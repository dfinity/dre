# Playbooks for common IC operations

## General common tasks

#### Check the status of a proposal

In a directory that has `dfx.json` configured for `mercury` (e.g. in `rs/nns/) run

```
dfx canister --no-wallet --network=mercury call governance get_proposal_info <proposal_number>
```

Alternatively, check proposals on https://ic.rocks/proposals

#### Get your neuron number

In a directory that has `dfx.json` configured for `mercury` (e.g. in `rs/nns/) run

```
dfx --identity hsm canister --no-wallet --network mercury call --query governance get_neuron_ids '()'
```

#### Update facts

```
testnet/release/factsdb/main.py --refresh
```

And commit the changes with a commit message something like "Updating mainnet facts after ...".

## Add a subnet #_i_

1. Select the hosts for the new subnet via https://dashboard.mercury.dfinity.systems/ or the IC Topology spreadsheet.
2. Check the guests by running:
```
testnet/release/ops/ic_guest_check.py --deployment-name mercury --nodes node1 [node2]...
```
for example:
```
testnet/release/ops/ic_guest_check.py --deployment-name mercury --nodes fm1-dll11 at1-spm04 zh2-spm05 br1-dll18 pl1-dll11 sg1-dll03 an1-dll14
```
3. Update `testnet/env/mercury/hosts.ini` to create a new subnet and move selected nodes to it.
4. Propose subnet creation:
   * `./mainnet-op propose-to-create-subnet <subnet-number> <subnet-type> <version-commit>`
5. Advertise the proposal, and watch dashboards when the proposal is being executed.
6. Merge the updated facts after topology changes.

## Add node(s) to a subnet

1. Determine `<subnet_number>` of the subnet which should get new nodes, e.g. via
   * `./mainnet-op query get-subnet-list`
2. Get the `<node_id>` of each node to be added, e.g.:
    * `grep <host_name> testnet/release/factsdb/data/mercury_guests.csv`
3. Check the nodes to be added exist and are `unassigned` in the registry.  Record sub
    * `./mainnet-op query get-topology > topology_before.txt`
    * `./mainnet-op query get-node <node_id>`
    * `./mainnet-op query get-subnet <i> > subnet_before.txt`
5. Propose node addition:
    * `./mainnet-op  propose-to-add-nodes-to-subnet <subnet_number> <node_id> [<node_id_2> ...]`
6. After the proposal is executed, verify the nodes have been added to the subnet,
   and that they don't appear anymore as `unassigned` in the topology.
    * `./mainnet-op query get-subnet <i> > subnet_after.txt`
    * `diff subnet_before.txt subnet_after.txt`
    * `./mainnet-op query get-topology > topology_after.txt`
    * `diff topology_before.txt topology_after.txt`
7. Update hosts and facts after topology changes

## Remove node(s) from a subnet

1. Get the `<node_id>` of each node to be removed, e.g. via `guest_facts`:
    * `grep <host_name> testnet/env/mercury/guest_facts`
2. Check the nodes to be removed exist and are assigned to a subnet in the registry:
    * `./mainnet-op query get-subnet <i>`

    where subnet index `<i>` is taken from the `grep`-output from step #1.
3. Propose node removal:
    * `./mainnet-op  propose-to-remove-nodes-from-subnet <node_id> [<node_id_2> ...]`
4. After the proposal is executed, verify the nodes have been removed from the subnet,
   and that they now appear as `unassigned` in the topology.
   * `./mainnet-op query get-subnet <i>`
   * `./mainnet-op query get-topology > topology_after.txt`
5. Update hosts and facts after topology changes

## Remove nodes from the registry

1. Get the `<node_id>` of each node to be removed, e.g. via `guest_facts`:
   * `grep <host_name> testnet/env/mercury/guest_facts`
2. Check the nodes to be removed exist and are `unassigned` in the registry.
   * `./mainnet-op query get-topology > topology_before.txt`
   * `./mainnet-op query get-node <node_id>`
3. Propose node removal:
   * `./mainnet-op  propose-to-remove-nodes <node_id> [<node_id_2> ...]`
4. After the proposal is executed, verify the nodes have been removed.
    * `./mainnet-op query get-topology > topology_after.txt`
    * `diff topology_before.txt topology_after.txt`
    * `./mainnet-op query get-node <node_id>`

## Prepare and bless a new release


## Upgrade a subnet to a new release


## Upgrade NNS-subnet to a new release


