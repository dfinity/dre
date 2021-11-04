#!/bin/bash

GIT_REVISION="32d4e9c61c8b284d1bebed290df8d9b2efad2fc6"
RC_BRANCH="rc--2021-10-24_18-31"

for subnet in "1 (snjp4)" "3 (pae4o)" "11 (csyj4)" "19 (jtdsg)"; do
    SUBNUM=$(echo "$subnet" | cut -d' ' -f1)
    export MOTIVATION="Upgrade the subnet $subnet to $RC_BRANCH"
    ./mainnet-op propose-to-update-subnet-replica-version "$SUBNUM" "$GIT_REVISION"
done
