#!/usr/bin/env bash
#
# Clean up bazel cache if older than 7 days
#

set -eEuo pipefail

echo "Bazel cache directory contents:"
ls -l --full-time ~/.cache/bazel || exit 0  # In case bazel cache does not exist on github, it won't be mounted so we need to bail out here
find ~/.cache/bazel -mindepth 1 -maxdepth 1 -type d -mtime +7 -exec sudo rm -rvf '{}' +
