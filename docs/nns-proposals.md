# NNS/ic-admin operations

Most of the commands here can be run in multiple ways. Currently we are putting in the effort to make `dre` as useful as possible. As such it provides support for `dry_run` as default and that can be highly beneficial in most scenarios (for eg. if someone is asking you to submit a proposal for them the best practice way is to run a `dry_run` and ask them to double check the command and the payload that would be submitted) and that is why we recommend using `dre` whenever possible. In some use-cases `dre` cannot help you, and that is when you should use whatever tool/script is at hand.

### Get the principal from your HSM

```bash
❯ dfx identity use hsm
Using identity: "hsm".
❯ export DFX_HSM_PIN=$(cat ~/.hsm-pin)
❯ dfx identity get-principal
as4rt-t4nqh-64j36-ubyaa-2c6uz-f2qbm-67llh-ftd3y-epn2e-wzaut-wae
```

### Get the neuron id associated with your HSM

```bash
❯ export DFX_HSM_PIN=$(cat ~/.hsm-pin)
❯ dfx canister --identity=hsm --network=ic call rrkah-fqaaa-aaaaa-aaaaq-cai get_neuron_ids '()'
(vec { 40 : nat64 })
```

### Getting the Mainnet firewall rules

```bash
dre get firewall-rules replica_nodes | jq
```

### Get the Node Rewards Table, used for the Node Provider compensation

```bash
ic-admin --nns-url https://ic0.app get-node-rewards-table
{
  "table": {
    "Asia": {
[...]
}
```

- Alternative

```bash
dre get node-rewards-table
```

### Update the Node Rewards Table

```bash
ic-admin --nns-url https://ic0.app --use-hsm --pin $(cat ~/.hsm-pin) --key-id 01 --slot 0 propose-to-update-node-rewards-table --proposer $PROPOSER_NEURON_INDEX --summary-file 2022-12-type3.md --updated-node-rewards "$(cat 2022-12-type3-rewards.json)"
```

- Alternative

```bash
dre propose update-node-rewards-table --summary-file 2022-12-type3.md --updated-node-rewards "$(cat 2022-12-type3-rewards.json | jq -c)"
```

### Enable the HTTPs outcalls on a subnet

```bash
cargo run --bin ic-admin -- --nns-url https://ic0.app/ \	
	--use-hsm \
	--pin $(cat ~/.hsm-pin) \
	--key-id 01 \
	--slot 0 \
	propose-to-update-subnet \
	--proposer  <neuron>  \
	--features "http_requests" \
	--subnet uzr34-akd3s-xrdag-3ql62-ocgoh-ld2ao-tamcv-54e7j-krwgb-2gm4z-oqe \
	--summary "Enable the HTTPS outcalls feature on the non-whitelisted uzr34 subnet so that the exchange rate canister can query exchange rate data."
```

- Alternative

```bash
dre propose update-subnet \
	--features "http_requests" \
	--subnet uzr34-akd3s-xrdag-3ql62-ocgoh-ld2ao-tamcv-54e7j-krwgb-2gm4z-oqe \
	--summary "Enable the HTTPS outcalls feature on the non-whitelisted uzr34 subnet so that the exchange rate canister can query exchange rate data."
```

### Removing node operator principal id

```bash
dre propose remove-node-operators kdqam-hauon-sdvym-42eyg-5wyff-4ywbw-v6iij-2sw2z-bu4rj-ejusn-jae \
    --summary "<An appropriate summary for the proposal, and a link to the forum post for further discussion, if possible>"
```

### Removing nodes from the registry

Here is an example where we remove all AW1 nodes for redeployment.

```bash
dre nodes remove aw1 --motivation "Removing AW1 nodes for redeployment"
```
