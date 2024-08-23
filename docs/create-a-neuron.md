# Create a New Neuron for Proposal Submissions

Follow these steps to create a new neuron for proposal submission.

## Requirements

- 11 ICP (10 of which are to be staked for the NNS proposal deposit)
- Basic understanding of [neurons, staking, and governance proposals](https://internetcomputer.org/docs/current/tokenomics/nns/nns-staking-voting-rewards).
- Optional [Hardware wallet](https://www.ledger.com/)

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

The neuron in the NNS UI (https://nns.ic0.app) needs to be managed with a locally generated private key, in order to be able to submit proposals from the command line.
This is done through a so called Hotkey. In the NNS UI you need to add a locally generated identity as a hotkey, which means that you will be able to perform actions with the NNS UI identity, by using the locally generated identity.

```bash
dfx identity new --storage-mode=plaintext neuron-hotkey
```

```bash
dfx --identity neuron-hotkey identity get-principal
```
Example output: `wuyst-x5tpn-g5wri-mp3ps-vjtba-de3xs-w5xgb-crvek-tucbe-o5rqi-mae`

## (Optional) Hardware wallet identity

In the NNS UI you can also add a hardware wallet as a controlling entity. You will not be able to submit proposals with the hardware wallet entity though. But you can keep all your funds on the hardware wallet.

### Create and Manage Neuron via NNS Frontend Dapp and Internet Identity

1. Send at least 11 ICPs (10 of which are required for proposals submisison, and 1 is aiming to cover all potential fees) to your NNS UI address OR the hardware wallet address.
3. Navigate to the Neurons tab in the NNS UI and create a Neuron by staking at least 10 ICP. Confirm the transaction on your hardware wallet.
4. (Optional, if using hardware wallet) After the Neuron is created, confirm to "Add NNS Dapp as hotkey" in the dialogue and on your hardware wallet.
5. Set the dissolve delay to at least 6 months and confirm the choice.

That's it, you now have a local identity that can submit a proposal. If you need, you can add more ICPs using the same process.
