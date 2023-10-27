Deployment
==========

# Preflight
* Set up your HSM.
* Install `idl2json` from source: https://github.com/dfinity-lab/idl2json
* Install `rclone`: with `apt install rclone` or `brew install rclone` or `curl https://rclone.org/install.sh | sudo bash`

# Deployment

* Check out a release commit on master.
* In terminal 1:
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
    * Register nodes
      * `DEPLOY_INDEPENDENT_NODES=dada ./bootstrap.$NETWORK 9.[A-F]`
      * Then plug in the HSM
    * `APP_SUBNET_TYPE=SOME_STRING APP_SUBNET_NUMBER=NUMBER_FROM_1_TO_N ./bootstrap.$NETWORK 9`
    * Populate the hosts file
      * `cat ~/tmp/bootstrap/child_app_${APP_SUBNET_NUMBER}_hosts`
      * `vi ../../../../env/$NETWORK/hosts`

  * Deploy ic-fe and monitoring:
    * `./bootstrap.$NETWORK '1[0-9]'`
  * Exit the shell, to save the session.
