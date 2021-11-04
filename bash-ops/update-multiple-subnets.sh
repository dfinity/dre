#!/bin/bash

set -eEu

GIT_REVISION="32d4e9c61c8b284d1bebed290df8d9b2efad2fc6"
RC_BRANCH="rc--2021-10-24_18-31"

SUBNETS=()
SUBNETS+=("1 (snjp4)")
SUBNETS+=("3 (pae4o)")
SUBNETS+=("11 (csyj4)")
SUBNETS+=("19 (jtdsg)")

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

echo "Upgrade the following subnets to $RC_BRANCH ($GIT_REVISION)"
printf '%s\n' "${SUBNETS[@]}"
do_you_want_to_continue

for subnet in "${SUBNETS[@]}"; do
    SUBNUM=$(echo "$subnet" | cut -d' ' -f1)
    export MOTIVATION="Upgrade the subnet $subnet to $RC_BRANCH"
    yes | ./mainnet-op propose-to-update-subnet-replica-version "$SUBNUM" "$GIT_REVISION"
done
