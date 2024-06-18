#!/usr/bin/env bash

set -euo pipefail

echo "BUILD_TIME \"$(TZ=UTC date --rfc-3339=seconds)\""

if [[ -n "${GITHUB_SHA:-}" ]]; then
  GIT_REV="$GITHUB_SHA"
else
  GIT_REV=$(git tag --points-at HEAD)
  if [[ -z "${GIT_REV:-}" ]]; then GIT_REV=$(git describe --always --dirty); fi
fi

echo "GIT_REV ${GIT_REV:-unset}"

RELEASE_VERSION=$(grep ^version Cargo.toml | cut -d\" -f2)
echo "RELEASE_VERSION ${RELEASE_VERSION:-unset}"
