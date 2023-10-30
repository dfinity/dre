#!/usr/bin/sh

set -e

git_root=$(git rev-parse --show-toplevel)
(
    cd $git_root
    bazel run //:poetry -- export -f requirements.txt > requirements.txt.2
    mv requirements.txt.2 requirements.txt
)
