# Tools for IC-ops

## Prerequisites

Installing Python dependencies:
```bash
pip3 install -r requirements.txt
```

* for proposals (see [info on proposal parameters](https://github.com/dfinity/nns-proposals/tree/main/proposals)):
    * export envvar `PROPOSER_NEURON_INDEX`
    * set envvar `PROPOSAL_URL` (optional, if not set a generated default will be used)
    * set envvar `PROPOSAL_SUMMARY` (optional, if not set a generated default will be used)

* for **testnet** operations only:
    * export envvar `TESTNET` equal of the name of the testnet to be used
    * export envvar `PROPOSER_NAME` with a matching identity key in
      `$HOME/.config/dfx/identity/$TESTNET-$PROPOSER_NAME/identity.pem`, where $TESTNET is equal
      to the first argument of the script being run.
    * export envvar `NNS_URL`-variable pointing to an NNS-machine on the target IC

* for **mainnet** operations only:
    * export envvars `HSM_SLOT` and `KEY_ID`, identifying the HSM-device
      and the relevant key
    * HSM PIN in the file `$HOME/.hsm-pin`
    * export envvar `NNS_URL` pointing to an NNS-machine on the mainnet
      (optional, if not set, a default will be used)

* If automatic installation does not work, install `ic-admin`:

```bash
REPO_ROOT=$(git rev-parse --show-toplevel)
GIT_REVISION=$("$REPO_ROOT"/gitlab-ci/src/artifacts/newest_sha_with_disk_image.sh "origin/post-merge-tests-passed")
mkdir -p ~/bin
if [ -n "$GIT_REVISION" ]; then
  if [ "$(uname)" = "Darwin" ]; then
    curl https://download.dfinity.systems/blessed/ic/$GIT_REVISION/nix-release/x86_64-darwin/ic-admin.gz -o - | gunzip -c >| ~/bin/ic-admin
  else
    curl https://download.dfinity.systems/blessed/ic/$GIT_REVISION/release/ic-admin.gz -o - | gunzip -c >| ~/bin/ic-admin
  fi
  chmod +x ~/bin/ic-admin
fi
```

## Usage

### mainnet-ops

The basic usage is as follows (assuming the required envvars are set, see **Prerequisites** below):

`./mainnet-op <operation-name>  [other parameters]`

where `other parameters` depend on the operation to be executed. Run just `./mainnet-op`
(without parameters) to see which operations are supported, or run `./mainnet-op <operation-name>`
to see which parameters are needed for a specific operation.

Note that instead of `./mainnet-op` you can use `TESTNET=<deployment_name> ./testnet-op`

### ic_guest_check.py

This tool can be used to check if the nodes can be used in a network.

Usage:
```bash
./ic_guest_check.py --help
usage: ic_guest_check.py [-h] [--deployment-name DEPLOYMENT_NAME] [--node-filter NODE_FILTER] [--nodes NODES [NODES ...]] [--list-steps] [--step-filter STEP_FILTER [STEP_FILTER ...]] [--out-dir OUT_DIR] [--verbose]

optional arguments:
  -h, --help            show this help message and exit
  --deployment-name DEPLOYMENT_NAME
                        Deployment name (default: mercury)
  --node-filter NODE_FILTER
                        Filter for the deployment nodes, example: 'node_type=batch_1'
  --nodes NODES [NODES ...]
                        A list of nodes to run on.
  --list-steps, --ls, --dry-run
                        A list of steps.
  --step-filter STEP_FILTER [STEP_FILTER ...]
                        A regular expression filter for the steps to run.
  --out-dir OUT_DIR     The directory where the debug information should be written.
  --verbose, -v         Verbose mode
```

Example usage:
```bash
./ic_guest_check.py --deployment-name mercury --nodes fm1-dll11 at1-spm04 zh2-spm05 br1-dll18 pl1-dll11 sg1-dll03 an1-dll14
```

### Onboarding new nodes: ic_guest_prepare.py

Can be used to onboard new nodes to the network, running sanity checks on the physical hosts, and recreating the guest images.
At the moment it's necessary to edit `testnet/env/mercury/hosts.ini` and set the `node_type=onboarding` to the nodes that we are trying to onboard.

Usage:
```bash
./ic_guest_prepare.py --help
usage: ic_guest_prepare.py [-h] [--deployment-name DEPLOYMENT_NAME] [--node-filter NODE_FILTER] [--git-revision GIT_REVISION] [--list-steps] [--step-filter STEP_FILTER [STEP_FILTER ...]] [--out-dir OUT_DIR] [--verbose]

optional arguments:
  -h, --help            show this help message and exit
  --deployment-name DEPLOYMENT_NAME
                        Deployment name (default: mercury)
  --node-filter NODE_FILTER
                        Filter for the deployment nodes, example: 'node_type=batch_1'
  --git-revision GIT_REVISION
                        Git revision to deploy.
  --list-steps, --ls, --dry-run
                        A list of steps.
  --step-filter STEP_FILTER [STEP_FILTER ...]
                        A regular expression filter for the steps to run.
  --out-dir OUT_DIR     The directory where the debug information should be written.
  --verbose, -v         Verbose mode
```

Example usage:
```bash
./ic_guest_prepare.py --deployment-name mercury --node-filter node_type=onboarding --git-revision 27e1eadbcbe90abfe56d9c8dfd39e1a78e52c62
```

## Miscellaneous How-Tos

### How to find right values for `HSM_SLOT` and/or `KEY_ID`?

For `HSM_SLOT` run `pkcs11-tool --list-slots`, for `KEY_ID` run `pkcs11-tool --slot $HSM_SLOT --list-objects`.

### How to construct values for `PROPOSAL_URL`?

See [info on proposal parameters](https://github.com/ic-association/nns-proposals/tree/main/proposals).
