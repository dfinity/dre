Deployment
==========

# Preflight
* Set up your HSM.
* Get IC-Admin and SuperLeader key files.
* Having installed the above keys, your dfx config should look something like this:
```
max@sinkpad:~ (0)$ tree .config/dfx/identity
.config/dfx/identity
├── disaster-super-leader <- assuming that you will be deploying to the disaster recovery testnet
│   └── identity.pem <-- the super leader secret key.
├── disaster-support
│   └── identity.pem <-- the ic-admin secret key
├── my-nitro-key       <-- your HSM
│   ├── identity.json
│   └── wallets.json
```
* Install `idl2json` from source: https://github.com/dfinity-lab/idl2json
* Install `rclone`: with `apt install rclone` or `brew install rclone` or `curl https://rclone.org/install.sh | sudo bash`


# Deployment

* Check out a release commit on master.
* In terminal 1:
  * Setup:
    * `cd launch`
    * Remove the state from previous runs
      * `if test -e terminal.log ; then echo Saving terminal.log ; mkdir -p ~/tmp/bootstrap && mv terminal.log ~/tmp/bootstrap/terminal.log.$(date +%s) ; fi`
    * : Record terminal:
      * `script terminal.log`
    * `nix-shell`
    * `cd bootstrap`
  * `export NETWORK=<THE_NETWORK_NAME>`
  * Set the desired release commit and PR.
    * `vi bootstrap.$NETWORK`
  * Make sure that `/bin/bash` is present on the root node.
    * Note: This is a temporary requirement because we need a stable location for bash.  The root node is the only nix node, with bash in an inconsistent and unpredictable location.
  * Make super-sure that the node allowances are up to date.  Compare with mercury, for example.
  * Cleanup: `./bootstrap.$NETWORK -1`
  * Start:  `./bootstrap.$NETWORK '0'`
  * Scrutinise the environment variables: `./env`
    * WARNING: The ipv6 addresses are not coming out correctly in the `env_var`.
  * Scrutinise anything else that might be interesting in this run.
  * Deploy root node: `./bootstrap.$NETWORK 1`
  * Deploy parent nns nodes:
    * `./bootstrap.$NETWORK 2`
    * `DEPLOY_INDEPENDENT_NODES=dada ./bootstrap.$NETWORK 2`
      * Then plug in the HSM
    * Watch on a node with with:
      * `watch 'bash -c "hostname ; date ; sudo virsh list --all ; lsusb ; ls /tmp"'`
    * Ask the HSM to be plugged in
  * Deploy nns's: `./bootstrap.$NETWORK '[3-4]'`
  * Deploy child nns:
    * Create nodes
      * `DEPLOY_INDEPENDENT_NODES=dada ./bootstrap.$NETWORK 5.[A-F]`
        * Then plug in the HSM
      * `./bootstrap.$NETWORK 5.[A-F]`
    * create subnet
      * `./bootstrap.$NETWORK 5.[G-Z]`
    * Deply canisters
      * `./bootstrap.$NETWORK [67]`
  * Deploy app subnets:

    * `export APP_SUBNET_NUMBER=NUMBER_FROM_1_TO_N`
      * `DEPLOY_INDEPENDENT_NODES=dada ./bootstrap.$NETWORK 9.[A-F]`
        * Then plug in the HSM
      * `./bootstrap.$NETWORK 9.[A-F]`
      * `./bootstrap.$NETWORK 9.[G-Z]`
    * Populate the hosts file
      * `cat ~/tmp/bootstrap/child_app_${APP_SUBNET_NUMBER}_hosts`
      * `vi ../../../../env/$NETWORK/hosts`

  * Deploy ic-fe and monitoring:
    * `./bootstrap.$NETWORK '1[0-9]'`
  * Exit the shell, to save the session.
