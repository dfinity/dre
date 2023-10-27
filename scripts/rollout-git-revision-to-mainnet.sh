#!/usr/bin/env bash

set -eEuo pipefail

GIT_REVISION_REGULAR=3bcccef07408921fe849c92dd2437adc157ef9c3
export GIT_REVISION=${GIT_REVISION:-$GIT_REVISION_REGULAR}

SCRIPTS_PATH=$(
    cd "$(dirname "$0")"
    pwd
)
cd "$SCRIPTS_PATH"

export PATH=$PATH:"$SCRIPTS_PATH/bin"

function countdown() {
    secs=$1
    while [ $secs -gt 0 ]; do
        echo -ne "Waiting $secs seconds\033[0K\r"
        sleep 1
        ((secs--))
    done
}

function wait_for_replica_revision() {
    SUBNET=$1
    REPLICA_REVISION=$2
    local subnet_node_ip=
    while true; do
        SUBNET_MEMBERS=$(./mainnet-op query get-subnet $SUBNET | jq -e '.records[0].value.membership[]' || true)
        if [[ -n "$SUBNET_MEMBERS" ]]; then
            SUBNET_NODES_IPS=$(echo "$SUBNET_MEMBERS" | xargs -I'{}' sh -c './mainnet-op query get-node {} | grep -o -E "ip_addr: \"([0-9a-z]{0,4}:){7}[0-9a-z]{0,4}\"" | head -n 1 | cut -c10-')
            if [[ -n "$SUBNET_NODES_IPS" ]]; then
                local all_nodes_upgraded=1
                for subnet_node_ip in $SUBNET_NODES_IPS; do
                    subnet_node_ip=$(echo "[$subnet_node_ip]" | sed 's/"//g') # sorry, it has quotes
                    local ret=0
                    local node_data=$(timeout 30 curl --silent --fail http://$subnet_node_ip:9090/metrics) || ret=$?
                    if [ "$ret" = "0" ]; then
                        if echo "$node_data" | grep "ic_active_version=.$REPLICA_REVISION" >/dev/null; then
                            true # >&2 echo "$subnet_node_ip has upgraded to $REPLICA_REVISION"
                        else
                            echo >&2 "$subnet_node_ip has *not yet* upgraded to $REPLICA_REVISION"
                            all_nodes_upgraded=0
                        fi
                    else
                        echo >&2 "$subnet_node_ip failed to be contacted via cURL"
                        all_nodes_upgraded=0
                    fi
                done
                if [ "$all_nodes_upgraded" = 1 ]; then
                    break
                fi
            fi
        fi
        echo "$(date -uR): Replica version $REPLICA_REVISION still not active on subnet $SUBNET, waiting"
        countdown 60
    done
    echo "$(date -uR): Replica revision $REPLICA_REVISION is active on all nodes in the subnet $SUBNET"
}

SUBNET_IDS=($(./mainnet-op query get-subnet-list | jq -e -r '.[]'))
function get_subnet_replica_version() {
    SUBNET_NUM=$1
    SUBNET=${SUBNET_IDS[$SUBNET_NUM]}
    while true; do
        SUBNET_VERSION=$(timeout 30 curl --silent --fail https://dashboard.internal.dfinity.network/api/proxy/registry/mainnet/subnets \
            | jq -e -r 'to_entries[] | "\(.key)\t\(.value.replica_version)"' \
            | grep "^$SUBNET"$'\t' | cut -d$'\t' -f2)
        if [[ -n "$SUBNET_VERSION" ]]; then
            echo $SUBNET_VERSION
            break
        fi
        echo >&2 "Got an empty subnet version, retrying in a bit"
        sleep 3
    done
}

function subnet_check_for_alerts() {
    SUBNET_NUM=$1
    SUBNET_ID=${SUBNET_IDS[$SUBNET_NUM]}
    timeout 30 curl -G https://prometheus.mainnet.dfinity.network/api/v1/query -fsSL -m 30 \
        --retry 10 --retry-connrefused -H 'Accept: application/json' \
        --data-urlencode "query=sum_over_time(ALERTS{ic_subnet=\"$SUBNET_ID\", alertstate=\"firing\", severity=\"page\"}[30m])" \
        | jq '.data.result[]'
}

function wait_until_no_alerts_on_subnets() {
    for s in "$@"; do
        subnet_alerts=$(subnet_check_for_alerts $s)
        while [[ -n "$subnet_alerts" ]]; do
            echo "$(date -uR): Subnet $s has firing alerts"
            # Send some bells to the terminal
            for i in {0..4}; do
                tput bel
                sleep 1
            done
            echo "$subnet_alerts"
            countdown $((5 * 60))
            subnet_alerts=$(subnet_check_for_alerts $s)
        done
    done
}

function subnet_check_open_proposals() {
    SUBNET_NUM=$1
    SUBNET_ID=${SUBNET_IDS[$SUBNET_NUM]}
    HAD_PROPOSALS=false
    while timeout 30 curl --silent --fail -X GET "https://ic-api.internetcomputer.org/api/v3/proposals?limit=50&include_status=OPEN&offset=0" -H "accept: application/json" | jq '.data[] | select(.action_nns_function == "UpdateSubnetReplicaVersion")' | grep "$SUBNET_ID" >/dev/null; do
        echo "$(date -uR): Subnet $SUBNET_NUM ($SUBNET_ID) has open proposals. Waiting."
        HAD_PROPOSALS=true
        countdown 60
    done
    echo "$(date -uR): Subnet $SUBNET_NUM ($SUBNET_ID) does NOT have open proposals."
    if $HAD_PROPOSALS; then
        # Subnet had open proposals, check if the new version is the target version
        if [[ "$(get_subnet_replica_version $SUBNET_NUM)" == "$GIT_REVISION" ]]; then
            # Also check if all nodes in the subnet finished upgrading
            wait_for_replica_revision $SUBNET_NUM $GIT_REVISION
            echo "$(date -uR): Waiting a bit more before continuing"
            countdown $((5 * 60))
            wait_until_no_alerts_on_subnets $SUBNET_NUM
        fi
    fi
}

function update_subnets() {
    echo "To interrupt press Ctrl+C."
    countdown 10
    echo y | ./update-subnets.sh "$@"
    echo "$(date -uR): Updated subnets $@"
    for s in "$@"; do
        wait_for_replica_revision $s $GIT_REVISION
    done
}

function wait_until_day_of_week_and_hour() {
    WAIT_DOW=$1
    WAIT_HOUR=$2
    if [[ -z "$WAIT_DOW" || -z "$WAIT_HOUR" ]]; then
        echo "$(date -uR): Invalid day-of-week '$WAIT_DOW' or hour '$WAIT_HOUR' provided."
        exit 1
    fi
    while true; do
        DOW=$(date -u +%u)
        HOUR=$(date -u +%-H)
        if ((DOW > WAIT_DOW)); then
            echo "$(date -uR): Day of week '$DOW' > '$WAIT_DOW'. Proceeding."
            break
        fi
        if ((DOW >= WAIT_DOW && HOUR >= WAIT_HOUR)); then
            echo "$(date -uR): Day of week '$DOW' >= '$WAIT_DOW' and hour '$HOUR' >= '$WAIT_HOUR'. Proceeding."
            break
        fi
        if ((DOW < WAIT_DOW)); then
            echo "$(date -uR): Day of week '$DOW' still lower than '$WAIT_DOW'. Waiting."
            countdown 3600
        fi
        if ((HOUR < WAIT_HOUR)); then
            echo "$(date -uR): Hour '$HOUR' still lower than '$WAIT_HOUR' UTC. Waiting."
            countdown 60
        fi
    done
}

function update_subnets_and_wait_long() {
    WAIT_DAY=$1
    shift
    WAIT_HOUR=$1
    shift
    for s in "$@"; do
        subnet_check_open_proposals $s
    done
    UPDATE=false
    for s in "$@"; do
        SUBNET_REVISION=$(get_subnet_replica_version $s)
        if [[ "$SUBNET_REVISION" == "$GIT_REVISION" ]]; then
            echo "$(date -uR): Subnet $s already at revision $GIT_REVISION"
        else
            echo "$(date -uR): Subnet $s running at revision $SUBNET_REVISION != $GIT_REVISION. Will update."
            UPDATE=true
        fi
    done
    if $UPDATE; then
        wait_until_day_of_week_and_hour $WAIT_DAY $WAIT_HOUR
        update_subnets "$@"
        countdown $((30 * 60))
        wait_until_no_alerts_on_subnets "$@"
    fi
}

function update_subnets_and_wait_short() {
    WAIT_DAY=$1
    shift
    WAIT_HOUR=$1
    shift
    for s in "$@"; do
        subnet_check_open_proposals $s
    done
    UPDATE=false
    for s in "$@"; do
        SUBNET_REVISION=$(get_subnet_replica_version $s)
        if [[ "$SUBNET_REVISION" == "$GIT_REVISION" ]]; then
            echo "$(date -uR): Subnet $s already at revision $GIT_REVISION"
        else
            echo "$(date -uR): Subnet $s running at revision $SUBNET_REVISION != $GIT_REVISION. Will update."
            UPDATE=true
        fi
    done
    if $UPDATE; then
        wait_until_day_of_week_and_hour $WAIT_DAY $WAIT_HOUR
        update_subnets "$@"
        countdown $((5 * 60))
        wait_until_no_alerts_on_subnets "$@"
    fi
}

function wait_nns_subnet_updated() {
    # This function waits until the NNS subnet is updated before proceeding with the rollout.
    # This is necessary because we want to ensure that the previous rollout is fully finished
    # before a new rollout is started.
    # However, the simple implementation below may not work in all cases.
    # For instance:
    # * The app subnet may run a version that should not be deployed to the NNS subnet
    # * The app subnet was already updated to the $GIT_REVISION
    # etc.
    # In these cases, please temporarily comment out the execution of this function
    # before starting the script, but make sure you don't commit/push that change.
    NNS_SUBNET_VERSION="$(get_subnet_replica_version 0)"
    APP_SUBNET_VERSION="$(get_subnet_replica_version 35)"
    while [[ "$NNS_SUBNET_VERSION" != "$APP_SUBNET_VERSION" ]]; do
        echo "$(date -uR): Waiting for the NNS subnet to be updated to $APP_SUBNET_VERSION"
        countdown $((10 * 60))
        NNS_SUBNET_VERSION="$(get_subnet_replica_version 0)"
        APP_SUBNET_VERSION="$(get_subnet_replica_version 35)"
    done
    echo "$(date -uR): NNS subnet already updated to $APP_SUBNET_VERSION"
}

function show_airflow_banner() {
    if [ -z "${bannershown-}" ]; then
        echo "                                                                                        "
        echo "  ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓"
        echo "  ┃                                                                                    ┃"
        echo -e "  ┃   \e[1mDid you know there is an easier way to do this?\e[0m                                  ┃"
        echo "  ┃                                                                                    ┃"
        echo "  ┃   Use the new rollout Airflow flow to create proposals, monitor the network, and   ┃"
        echo "  ┃   make progress on the rollout, without needing to keep this script running        ┃"
        echo "  ┃   somewhere.                                                                       ┃"
        echo "  ┃                                                                                    ┃"
        echo "  ┃   Visit https://www.notion.so/dfinityorg/Weekly-IC-OS-release-using-Airflow to     ┃"
        echo "  ┃   learn how the new rollout process works, and then use our Airflow instance at    ┃"
        echo "  ┃   https://airflow.ch1-obsdev1.dfinity.network/home to get going.                   ┃"
        echo "  ┃                                                                                    ┃"
        echo "  ┃   Happy hacking!                                                                   ┃"
        echo "  ┃                                                                                    ┃"
        echo "  ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛"
        echo "                                                                                        "
        bannershown=true
    fi
}

# Allow operator to run subcommands defined here as functions.
if [ -n "${1-}" ]; then
    cmd="$1"
    shift
    ret=0
    "$cmd" "$@" || ret=$?
    exit "$ret"
fi

show_airflow_banner
wait_nns_subnet_updated

echo "┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄"
echo "  Upgrading the network to the git revision $GIT_REVISION"
echo -e "  \e[1mTo stop, press Ctrl+C now.\e[0m"
show_airflow_banner
echo "┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄"

export FORCE_GIT_REVISION=y

# Daily batch start times
RUN_HOUR_BATCH1=7  #  7:00 UTC
RUN_HOUR_BATCH2=9  #  9:00 UTC
RUN_HOUR_BATCH3=11 # 11:00 UTC
RUN_HOUR_BATCH4=13 # 13:00 UTC

# Monday
RUN_DAY=1 # The day when the deployment starts, you can edit this variable to start the deployment on a different day.
update_subnets_and_wait_long $RUN_DAY $RUN_HOUR_BATCH2 6
update_subnets_and_wait_long $RUN_DAY $RUN_HOUR_BATCH3 8 33

# Tuesday
RUN_DAY=2
update_subnets_and_wait_long $RUN_DAY $RUN_HOUR_BATCH1 15 18
update_subnets_and_wait_long $RUN_DAY $RUN_HOUR_BATCH2 1 5 2
update_subnets_and_wait_long $RUN_DAY $RUN_HOUR_BATCH3 4 9 34

# Wednesday
RUN_DAY=3
update_subnets_and_wait_long $RUN_DAY $RUN_HOUR_BATCH1 3 7 11
update_subnets_and_wait_long $RUN_DAY $RUN_HOUR_BATCH2 10 13 16
update_subnets_and_wait_long $RUN_DAY $RUN_HOUR_BATCH3 20 27 24
update_subnets_and_wait_long $RUN_DAY $RUN_HOUR_BATCH4 21 12 28

# Thursday
RUN_DAY=4
update_subnets_and_wait_long $RUN_DAY $RUN_HOUR_BATCH1 26 22 23
update_subnets_and_wait_long $RUN_DAY $RUN_HOUR_BATCH2 25 29 19
update_subnets_and_wait_long $RUN_DAY $RUN_HOUR_BATCH3 17 32 35
update_subnets_and_wait_long $RUN_DAY $RUN_HOUR_BATCH4 30 31 14

# Friday
RUN_DAY=5

# Following Monday

RUN_DAY=1
# Issued manually
# update_subnets_and_wait_long $RUN_DAY $RUN_HOUR_BATCH3 0 # NNS
