#!/usr/bin/env bash

files=("release-index-schema.json" "release-index.yaml" "release-controller/ci_check.py")

found=false

for file in "$@"; do
    if [[ "${files[@]}" =~ "$file" ]]; then
        found=true
        break
    fi
done

if [[ "$found" == false ]]; then
    exit 0
fi
command -v rye >/dev/null || echo "'rye' not found. Please install it by following the instructions from https://rye.astral.sh/"
rye run python release-controller/ci_check.py --repo-path ~/.cache/git/ic
