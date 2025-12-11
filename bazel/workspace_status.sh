#!/usr/bin/env bash

set -euo pipefail

# date format is hardcoded with +00:00 since we define UTC
echo "BUILD_TIME \"$(TZ=UTC date -u +"%Y-%m-%d %H:%M:%S+00:00")\""

if [[ -n "${GITHUB_SHA:-}" ]]; then
  GIT_REV="$GITHUB_SHA"
else
  GIT_REV=$(git tag --points-at HEAD)
  if [[ -z "${GIT_REV:-}" ]]; then GIT_REV=$(git describe --always --dirty); fi
fi

echo "GIT_REV ${GIT_REV:-unset}"

RELEASE_VERSION=$(grep ^version Cargo.toml | cut -d\" -f2)
echo "RELEASE_VERSION ${RELEASE_VERSION:-unset}"
