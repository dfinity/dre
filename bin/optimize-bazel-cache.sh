#!/usr/bin/env bash
#
# Optimize the size of bazel cache
#

set -eExuo pipefail

echo "Bazel cache directory contents:"
ls -l --full-time ~/.cache/bazel || exit 0 # In case bazel cache isn't mounted yet on github, we need to stop here

du -sh --total ~/.cache/bazel/*

bazel clean
du -sh --total ~/.cache/bazel/*

sudo apt install -qy rdfind
sudo rdfind -makehardlinks true "$(bazel info output_base | grep .cache | tail -n1)"
du -sh --total ~/.cache/bazel/*
