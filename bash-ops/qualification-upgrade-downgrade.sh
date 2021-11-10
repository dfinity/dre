#!/usr/bin/env bash

IC_REPO_PATH=$HOME/projects/dfinity
RELEASE_CLI_REPO_PATH=$HOME/projects/release-cli

cd "$IC_REPO_PATH"/testnet
MAINNET_REV_INITIAL=f2fc23733b52c53c8f1cfc05eba508c05e87c8b1
MAINNET_REV_OLD_NNS=32d4e9c61c8b284d1bebed290df8d9b2efad2fc6
MAINNET_REV_OLD_APP=32d4e9c61c8b284d1bebed290df8d9b2efad2fc6
MAINNET_REV_NEW=3a1286e844ed3b53b1848533cb844ea29b508f28

export TESTNET=medium07
export PATH=~/bin:$PATH:~/projects/release-cli/bash-ops

# read -r -p "'lock $TESTNET' with @Dee (Slack) and press ENTER to continue" response

./tools/icos_deploy.sh --dkg-interval-length 19 --git-revision $MAINNET_REV_INITIAL $TESTNET --ansible-args "-e initial_neurons=$RELEASE_CLI_REPO_PATH/deployments/env/bootstrap/initial-neurons.csv"
mkdir -p ~/bin
curl https://download.dfinity.systems/blessed/ic/$MAINNET_REV_OLD_NNS/release/ic-admin.gz -o - | gunzip -c >|~/bin/ic-admin.$MAINNET_REV_OLD_NNS
chmod +x ~/bin/ic-admin.$MAINNET_REV_OLD_NNS
curl https://download.dfinity.systems/blessed/ic/$MAINNET_REV_NEW/release/ic-admin.gz -o - | gunzip -c >|~/bin/ic-admin.$MAINNET_REV_NEW
chmod +x ~/bin/ic-admin.$MAINNET_REV_NEW
ln -sf ~/bin/ic-admin.$MAINNET_REV_NEW ~/bin/ic-admin

function wait_for_replica_revision() {
    SUBNET=$1
    REPLICA_REVISION=$2
    while true; do
        SUBNET_MEMBERS=$(testnet-op query get-subnet $SUBNET | jq '.records[0].value.membership[]')
        if [[ -n "$SUBNET_MEMBERS" ]]; then
            SUBNET_NODES_IPS=$(echo "$SUBNET_MEMBERS" | xargs -I'{}' sh -c 'testnet-op query get-node {} | grep -o -E "ip_addr: \"([0-9a-z]{0,4}:){7}[0-9a-z]{0,4}\"" | head -n 1 | cut -c10-')
            if [[ -n "$SUBNET_NODES_IPS" ]]; then
                if echo "$SUBNET_NODES_IPS" | xargs -n1 -I_addr sh -c "curl -s http://[_addr]:9090/metrics | grep $REPLICA_REVISION"; then
                    break
                fi
            fi
        fi
        echo "New replica still not active on subnet $SUBNET, waiting"
        sleep 10
    done
    echo "New replica revision is active"
}
export MOTIVATION="Release qualification"
export CHANGELOG="* Test change"
yes | testnet-op propose-to-bless-replica-version $MAINNET_REV_OLD_NNS
if [[ "$MAINNET_REV_OLD_NNS" != "$MAINNET_REV_OLD_APP" ]]; then
    yes | testnet-op propose-to-bless-replica-version $MAINNET_REV_OLD_APP
fi
yes | testnet-op propose-to-bless-replica-version $MAINNET_REV_NEW
yes | testnet-op propose-to-update-subnet-replica-version 0 $MAINNET_REV_OLD_NNS
yes | testnet-op propose-to-update-subnet-replica-version 1 $MAINNET_REV_OLD_APP
wait_for_replica_revision 0 $MAINNET_REV_OLD_NNS
wait_for_replica_revision 1 $MAINNET_REV_OLD_APP
yes | testnet-op propose-to-update-subnet-replica-version 1 $MAINNET_REV_NEW
wait_for_replica_revision 1 $MAINNET_REV_NEW

# Check testnet subnet in Grafana
echo "Check in Browser: https://grafana.dfinity.systems/d/q9w4oZWGz/ic-progress-clock?orgId=1&var-ic=$TESTNET&var-ic_subnet=All&var-instance=All&var-period=$__auto_interval_period&refresh=30s"
echo "Check in Browser: https://grafana.dfinity.systems/d/GWlsOrn7z/execution-metrics-2-0?orgId=1&var-ic=$TESTNET&var-ic_subnet=All&var-instance=All&var-node_instance=All&var-heatmap_period=$__auto_interval_heatmap_period"

yes | testnet-op propose-to-update-subnet-replica-version 0 $MAINNET_REV_NEW
wait_for_replica_revision 0 $MAINNET_REV_NEW

# Check testnet subnet in Grafana

export GIT_REVISION=$MAINNET_REV_NEW
curl https://download.dfinity.systems/blessed/ic/$MAINNET_REV_NEW/release/ic-workload-generator.gz -o - | gunzip -c >|~/bin/ic-workload-generator
chmod +x ~/bin/ic-workload-generator
# comment out icos_deploy.sh line
sed -i.bak 's/^deploy_with_timeout/#[release team disabled] deploy_with_timeout/' tests/scripts/generic.sh
./tests/scripts/generic.sh $TESTNET 120 100 1k ../artifacts/results-$TESTNET-$(date --iso-8601)

#
# Downgrade and verify success
#
yes | testnet-op propose-to-update-subnet-replica-version 1 $MAINNET_REV_OLD_APP
yes | testnet-op propose-to-update-subnet-replica-version 0 $MAINNET_REV_OLD_NNS
wait_for_replica_revision 1 $MAINNET_REV_OLD_APP
wait_for_replica_revision 0 $MAINNET_REV_OLD_NNS
# Wait for the update to the old revision succeeds
./tests/scripts/generic.sh $TESTNET 120 100 1k ./results-$TESTNET-$(date --iso-8601)

# Check testnet subnet in Grafana
echo "Check in Browser: https://grafana.dfinity.systems/d/q9w4oZWGz/ic-progress-clock?orgId=1&from=now-1h&to=now&var-ic=$TESTNET&var-ic_subnet=All&var-instance=All&var-period=$__auto_interval_period"
echo "Check in Browser: https://grafana.dfinity.systems/d/GWlsOrn7z/execution-metrics-2-0?orgId=1&var-ic=$TESTNET&var-ic_subnet=All&var-instance=All&var-node_instance=All&var-heatmap_period=$__auto_interval_heatmap_period"
