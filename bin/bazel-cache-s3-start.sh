#!/usr/bin/env bash

set -eExuo pipefail

mkdir -p ~/.cache/bazel-remote
docker run --detach --name=bazel-remote -u $(id -u):$(id -g) \
    -v ~/.cache/bazel-remote:/data -v $HOME/.aws:/aws-config \
    -p 9090:8080 -p 9092:9092 \
    quay.io/bazel-remote/bazel-remote \
    --max_size 30 \
    --s3.auth_method=aws_credentials_file --s3.aws_profile=wasabi --s3.aws_shared_credentials_file=/aws-config/credentials --s3.bucket=dre-ci-cache --s3.endpoint=s3.eu-central-2.wasabisys.com
