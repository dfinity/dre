#!/bin/bash

GIT_REVISION="32d4e9c61c8b284d1bebed290df8d9b2efad2fc6"
RC_BRANCH="rc--2021-10-24_18-31"

for subnet in "4 (4zbus)" "9 (ejbmu)" "10 (eq6en)" "7 (5kdm2)"; do
    SUBNUM=$(echo "$subnet" | cut -d' ' -f1)
    export MOTIVATION="Upgrade the subnet $subnet to $RC_BRANCH"
    ./mainnet-op propose-to-update-subnet-replica-version "$SUBNUM" "$GIT_REVISION"
done
