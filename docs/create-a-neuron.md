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

### Create and Manage Neuron via NNS Frontend Dapp and Internet Identity

1. Send at least 11 ICPs (10 of which are required for proposals submisison, and 1 is aiming to cover all potential fees) to your NNS UI address.
2. Navigate to the Neurons tab in the NNS UI and create a Neuron by staking at least 10 ICP.
3. Set the dissolve delay to at least 6 months and confirm the choice.

## Create a neuron Hotkey

The neuron in the NNS UI (https://nns.ic0.app) needs to be managed with a locally generated private key, in order to be able to submit proposals from the command line.
This is done through a so called Hotkey. In the NNS UI you need to add a locally generated identity as a hotkey, which means that you will be able to perform actions with the NNS UI identity, by using the locally generated identity.

```bash
dfx identity new --storage-mode=plaintext neuron-hotkey
dfx --identity neuron-hotkey identity get-principal
```
Example output: `wuyst-x5tpn-g5wri-mp3ps-vjtba-de3xs-w5xgb-crvek-tucbe-o5rqi-mae`

That's it, you now have a local identity that can submit proposals.

## Topping up a neuron

You may need to top up a neuron to send more proposals, either more proposals at the same time, or add balance in case some proposal gets rejected. You need 10 ICP per proposal submission.

The **recommended way** to add more ICPs to the neuron (increase the stake) is through the NNS UI. This should be straightforward and you can just follow the instructions in the NNS UI.

Topping up can also be done from the command line but is slightly more involved. We have built a helper utility for this:

```bash
dre neuron top-up
```
Will print you the neuron account address. Then you add funds to the printed account address, and after that you need to run:

```bash
dre neuron refresh
```

So that the staked balance on the neuron is refreshed by the governance canister. Refreshing is typically done automatically when topping up the neuron from the NNS UI.

## (Optional and advanced) Hardware wallet identity

In the NNS UI you can also add a hardware wallet as a controlling entity. You will not be able to submit proposals with the hardware wallet entity though. But you can keep all your funds on the hardware wallet.

Steps:
1. After the basic NNS neuron is created, add the hardware wallet in the NNS UI.
2. Confirm adding the IC NNS Dapp in your hardware wallet by following the instructions from the hardware wallet.
3. You can send funds to the hardware wallet address instead of sending them to the regular address in the NNS UI.
4. When staking ICPs, confirm the transaction on your hardware wallet.
5. For voting and proposal submission from the command line you still need to add a regular dfx identity (plaintext)
