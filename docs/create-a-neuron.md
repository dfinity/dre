# Instructions for Creating a New Neuron for Proposal Submission

Follow these steps to create a new neuron for proposal submission.

## Requirements

*   11 ICP (10 of which are to be staked for the NNS proposal deposit)
*   Basic understanding of [neurons, staking, and governance proposals](https://internetcomputer.org/docs/current/tokenomics/nns/nns-staking-voting-rewards).
*   Optional [Hardware wallet](https://www.ledger.com/)

## Install the Required Tools

Install dfx
```bash
sh -ci "$(curl -fsSL https://smartcontracts.org/install.sh)"
```

```bash
export PATH=$HOME/bin:$PATH
```

If you already had `dfx` installed, make sure it's updated to the latest version
```bash
dfx upgrade
dfx --version
```

## Create a neuron Hotkey

The neuron in the NNS UI (https://nns.ic0.app) needs to be managed with a locally generated private key. This is done though a so called Hotkey.
```bash
dfx identity new --storage-mode=plaintext neuron-hotkey
```

```bash
dfx --identity neuron-hotkey identity get-principal
```
Example output: `wuyst-x5tpn-g5wri-mp3ps-vjtba-de3xs-w5xgb-crvek-tucbe-o5rqi-mae`

**Note:** This hotkey is used for NNS proposal submissions only.

### Create and Manage Neuron via NNS Frontend Dapp and Internet Identity

1. Send at least 11 ICPs to your hardware wallet address.
2. Navigate to the Neurons tab and create a Neuron by staking at least 10 ICP from your hardware wallet. Confirm the transaction on your hardware wallet.
3. After the Neuron is created, confirm to "Add NNS Dapp as hotkey" in the dialogue and on your hardware wallet.
4. Set the dissolve delay to at least 6 months and confirm the choice.
5. Copy the Neuron ID from the Web UI, for use in the next steps.
