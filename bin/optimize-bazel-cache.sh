#!/usr/bin/env bash
#
# Optimize the size of bazel cache
#

set -eEuo pipefail

echo "Bazel cache directory contents:"
ls -l --full-time ~/.cache/bazel || exit 0  # In case bazel cache does not exist on github, it won't be mounted so we need to bail out here
du -sh --total ~/.cache/bazel/*
sudo apt install -qy rdfind
sudo rdfind -makehardlinks true "$(bazel info output_base | grep .cache | tail -n1)"
