#!/usr/bin/env sh

set -e

git_root=$(git rev-parse --show-toplevel)
(
    cd $git_root
    bazel run //:poetry -- export --with dev -f requirements.txt >requirements.txt.2
    mv requirements.txt.2 requirements.txt
)

bazel run //:poetry -- install
bazel run //:poetry -- run datamodel-codegen \
    --input $git_root/release-index-schema.json \
    --input-file-type jsonschema \
    --output $git_root/release-controller/release_index.py \
    --target-python-version 3.10 \
    --output-model-type pydantic_v2.BaseModel \
    --disable-timestamp
