#!/usr/bin/env bash

set -euo pipefail

if [[ ! -z "${CI_COMMIT_SHA+x}" ]]; then
    echo GIT_HASH "${CI_COMMIT_SHA}"
else
    echo GIT_HASH $(git rev-parse HEAD)
fi
