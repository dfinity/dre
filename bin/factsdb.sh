#!/usr/bin/env sh

set -e

export GIT_ROOT=$(git rev-parse --show-toplevel)
(
    cd $GIT_ROOT
    bazel run //factsdb --  --deployment-name mainnet --refresh
    bazel run //factsdb --  --deployment-name staging --refresh
    if [ -n "$(git status --short factsdb/data)" ]; then
        bazel run //factsdb --  --deployment-name mainnet --publish-guests
        bazel run //factsdb --  --deployment-name staging --publish-guests
    fi
)
