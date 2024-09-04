#!/usr/bin/env bash

set -eEuo pipefail

git_root=$(git rev-parse --show-toplevel)
cd "$git_root"
command -v rye >/dev/null \
    || rye run datamodel-codegen \
        --input $git_root/release-index-schema.json \
        --input-file-type jsonschema \
        --output $git_root/release-controller/release_index.py \
        --target-python-version 3.12 \
        --output-model-type pydantic_v2.BaseModel \
        --disable-timestamp
