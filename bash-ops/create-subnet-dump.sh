#!/bin/bash

set -eEuo pipefail

curl --fail --silent https://dashboard.mercury.dfinity.systems/api/proxy/registry/create_subnet\?size\=13  | jq -r  '.[] | .hostname' | tr '\n' ' ' | xargs ./decentralization-dump.sh none
