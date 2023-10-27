#!/bin/bash
# A script to update multiple subnets to the specified git revision
#
# Usage: ./update-subnets.sh 5 6
# (update subnets 5 and 6)
set -eEuo pipefail

GIT_REVISION="${GIT_REVISION:-e00f758c89d5c37b794dec308c0444a99ac1e9f7}"
SCRIPTS_PATH=$(
    cd "$(dirname "$0")"
    pwd
)
cd "$SCRIPTS_PATH"

# An array of short subnet IDs (first 5 chars)
SUBNET_IDS=($(./mainnet-op query get-subnet-list | jq -r '.[]'))
SUBNET_SHORT=($(./mainnet-op query get-subnet-list | jq -r '.[] | split("-")[0]'))

function get_subnet_replica_version() {
    SUBNET_NUM=$1
    SUBNET=${SUBNET_IDS[$SUBNET_NUM]}
    curl --silent https://dashboard.internal.dfinity.network/api/proxy/registry/mainnet/subnets \
        | jq -r 'to_entries[] | "\(.key)\t\(.value.replica_version)"' \
        | grep "^$SUBNET"$'\t' | cut -d$'\t' -f2
}

print_red() {
    echo -e "\033[0;31m$*\033[0m" 1>&2
}

print_green() {
    echo -e "\033[0;32m$*\033[0m"
}
do_you_want_to_continue() {
    read -r -p "Do you want to continue? [y/N] " response
    if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
        print_green "continuing..."
    else
        print_red "aborting..."
        exit 1
    fi
}

SUBNETS_TO_UPDATE=()
for i in "$@"; do
    if [[ "$(get_subnet_replica_version $i)" == "$GIT_REVISION" ]]; then
        print_red "Refusing to upgrade subnet $i to the same revision it's already running."
        continue
    else
        SUBNETS_TO_UPDATE+=("$i (${SUBNET_SHORT[$i]})")
    fi
done

echo "Upgrade the following subnets to git revision $GIT_REVISION"
printf '%s\n' "${SUBNETS_TO_UPDATE[@]}"
do_you_want_to_continue

LAST_BLESSED_REVISION="$(./mainnet-op query get-blessed-replica-versions | tr ',' '\n' | tail -n1 | grep -o -E '[a-z0-9]+')"
if [[ "$GIT_REVISION" != "$LAST_BLESSED_REVISION" ]]; then
    print_red "Trying to upgrade to a revision that is not the last blessed revision"
    if [[ "${FORCE_GIT_REVISION:-}" != "y" ]]; then
        do_you_want_to_continue
    fi
fi

for subnet in "${SUBNETS_TO_UPDATE[@]}"; do
    SUBNUM=$(echo "$subnet" | cut -d' ' -f1)
    echo "In 5 seconds: Upgrade subnet $SUBNUM to git revision $GIT_REVISION"
    sleep 5
    echo y | ./mainnet-op propose-to-update-subnet-replica-version "$SUBNUM" "$GIT_REVISION"
done
