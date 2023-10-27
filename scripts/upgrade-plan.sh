#!/bin/bash

# Generates the rollout plan based on the backend/dashboard data. E.g.:
# GIT_REVISION=04fe8b0a1262f07c0cec1fdfa838a37607370a61 ./update-subnets.sh 6
# GIT_REVISION=04fe8b0a1262f07c0cec1fdfa838a37607370a61 ./update-subnets.sh 1 8
# GIT_REVISION=04fe8b0a1262f07c0cec1fdfa838a37607370a61 ./update-subnets.sh 5 15
# [...]
curl https://dashboard.mercury.dfinity.systems/api/proxy/registry/rollout | jq -r --argfile a1 <(ic-admin --nns-url https://ic0.app get-subnet-list) '.release as $r | .stages[] | [.subnets[] | .principal | . as $p | $p | [$a1 | to_entries| .[] | {key: .value, value: .key}] | from_entries | .[$p]  ] | join(" ") | "GIT_REVISION=" + $r.commit_hash + " ./update-subnets.sh " + .'
