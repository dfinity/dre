
# Getting the current Node Rewards

```
ic-admin --nns-url https://ic0.app get-node-rewards-table
{
  "table": {
    "Asia": {
[...]
}
```

# Updating the Node Rewards

```
cd /this/folder/..
source lib.sh
cd -
DFX_HSM_PIN=$(load_hsm_pin) && \
ic-admin --nns-url https://ic0.app --use-hsm --pin "$DFX_HSM_PIN" --key-id 01 --slot 0 propose-to-update-node-rewards-table --proposer $PROPOSER_NEURON_INDEX --summary-file 2022-12-type3.md --updated-node-rewards "$(cat 2022-12-type3-rewards.json | jq -c)" --dry-run
```

If you're happy with the payload, remove the `--dry-run` and rerun.

