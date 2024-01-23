
# Get trustworthy metrics from the IC Mainnet

## Introduction and prerequisites

To be able to fetch trustworthy metrics there are a couple of things needed prior to running this extension:

1. You need a dfx principal. If needed you can create a new one with
    
    ```bash
    # You can use the one from your HSM but there are some caveats to that that will be addressed later
    dfx identity new <identity-name>
    ```

    or follow instructions from the [IC SDK Docs](https://internetcomputer.org/docs/current/developer-docs/setup/cycles/cycles-wallet/#creating-a-cycles-wallet-on-the-mainnet)
    
2. You can list available dfx identities with `dfx identity list` and then need to select that identity and get its principal.
    
    ```bash
    dfx identity use <dfx-identity-name>
    dfx identity get-principal
    ```
    
3. Check the current balance for the principal
    
    ```bash
    dfx ledger --network ic balance
    ```
    
    If you have less than 2 Trillion Cycles (TC) worth of ICP, based on the [current ICP value](https://www.coinbase.com/converter/icp/xdr), you can top up the ICP balance by sending funds to the principal, e.g., from [https://ic0.app/wallet/](https://ic0.app/wallet/).
    
    1 TC corresponds to 1 XDR at the time of conversion. XDR is the currency symbol of the IMF SDR, a basket of five fiat currencies, corresponding to 1.33 U.S. dollar at the time of writing. Canister creation itself will cost 1 TC, and you will need some cycles more to execute commands.
    
4. Create the wallet canister, after that you will get the wallet canister id in the output.
    
    ```bash
    dfx ledger --network ic create-canister --amount 0.5 <principal-from-step-2>
    ```
    
    You may need to adjust the amount of ICPs if needed, based on the current ICP value. More info can be found in the [IC SDK Docs](https://internetcomputer.org/docs/current/references/cli-reference/dfx-ledger/#options).
    
5. Deploy the wallet canister code
    
    ```bash
    dfx identity --network ic deploy-wallet <wallet-canister-id-from-step-4>
    ```
    

### Using the cli

You can obtain the DRE tool by following [getting started](../getting-started.md)

To test out the command you can run the following command

```bash
dre <key-params> trustworthy-metrics <wallet-canister-id> <start-at-timestamp> [<subnet-id>...]
```

Arguments explanation:

1. `wallet-canister-id` - id of the created wallet canister created in the step 4 above, or obtained by
   ```bash
    dfx identity --network ic get-wallet
    ```
2. `start-at-timestamp` - used for filtering the output. To get all metrics, provide 0
3. `subnet-id` - subnets to query, if empty will provide metrics for all subnets
4. `key-params` - depending on which identity you used to deploy the wallet canister you have two options:

If you used a purely new identity (which is advised since the tool can then parallelise the querying) you have to:

1. export identity as `.pem` file which you can do as follows:
    
    ```bash
    dfx identity export <identity-name> > identity.pem
    ```
    
2. replace `<key-params>` in the command with something like: `--private-key-pem identity.pem`

If you used an HSM then replace `<key-params>` with: `--hsm-slot 0 --hsm-key-id 0 --hsm-pin $(cat <pin-file>)`. Note that the HSM is less parallel than the key file due to hardware limits, so getting metrics with an HSM will be a bit slower.

Even if created the wallet canister with an HSM you can still add another file-based controller to the wallet canister:

1. Get the principal of new identity
    
    ```bash
    dfx identity use <identity-name> && dfx identity get-principal
    ```
    
2. Add the identity as the controller of canister
    
    ```bash
    dfx identity use <identity-name-used-for-creating-canister>
    dfx wallet --network ic add-controller <principal>
    ```
    
3. Use the newly created identity while running the tool.


# Example use

Here are some real-world examples of how metrics can be retrieved:

```bash
dre --private-key-pem identity.pem trustworthy-metrics nanx4-baaaa-aaaap-qb4sq-cai 0 > data.json
```

Or with an HSM:
```bash
dre --hsm-slot 0 --hsm-key-id 0 --hsm-pin "<pin>" trustworthy-metrics nanx4-baaaa-aaaap-qb4sq-cai 0 > data.json
```

You can check some examples of the analytics possible with the IC Mainnet data in the following [Jupyter Notebook](./TrustworthyMetricsAnalytics.ipynb)

If you don't have Jupyter notebooks locally, you can use [Google Colaboratory](https://colab.research.google.com/github/dfinity/dre/blob/main/docs/trustworthy-metrics/TrustworthyMetricsAnalytics.ipynb) or [![Binder](https://mybinder.org/badge_logo.svg)](https://mybinder.org/v2/gh/dfinity/dre/main?labpath=docs%2Ftrustworthy-metrics%2FTrustworthyMetricsAnalytics.ipynb) to run it online for free.
