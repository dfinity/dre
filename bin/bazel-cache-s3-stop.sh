#!/usr/bin/env bash

set -eExuo pipefail

docker logs bazel-remote
docker stop bazel-remote
docker rm bazel-remote
