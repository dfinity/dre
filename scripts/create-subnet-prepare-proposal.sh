#!/bin/bash

set -eEuo pipefail

BASE_DIR="$(
    cd "$(dirname "$0")"
    pwd
)"
cd "$BASE_DIR"
NNS_GIT_REVISION=$(./mainnet-op query get-subnet 0 | jq -r '.records[0].value.replica_version_id')
SUBNET_VERSION=${SUBNET_VERSION:-$NNS_GIT_REVISION}
SUBNET_SIZE=${1:-13}
MIN_NAKAMOTO_NP=5.0
MIN_NAKAMOTO_COUNTRY=2.0
MIN_NAKAMOTO_AVERAGE=3.0
DASHBOARD_URL=https://dashboard.internal.dfinity.network # production
# DASHBOARD_URL=http://localhost:17000                    # dev/local

CREATE_SUBNET_OUTPUT=$(curl --silent --fail -XPOST $DASHBOARD_URL/api/proxy/registry/mainnet/subnet/create -H 'Content-Type: application/json' -d '{"size": '$SUBNET_SIZE',"min_nakamoto_coefficients":{"coefficients":{"node_provider":'$MIN_NAKAMOTO_NP',"country":'$MIN_NAKAMOTO_COUNTRY'},"average":'$MIN_NAKAMOTO_AVERAGE'}}')
NODE_IDS=($(echo "$CREATE_SUBNET_OUTPUT" | jq -r '.added[]'))
nodes="["
for node in "${NODE_IDS[@]}"; do
    nodes="$nodes\"$node\","
done
nodes="${nodes%,}]"

ALL_NODES=$(curl --fail --silent $DASHBOARD_URL/api/proxy/registry/mainnet/nodes | jq -r 'to_entries[] | .value')
# prints out detailed information about every node to be added

function jq_fields() {
    echo "$ALL_NODES" | jq --argjson nodes "$nodes" 'select(.principal | IN($nodes[])) | '"$1"
}

jq_fields '{ hostname: .hostname, dirty: .dirty, principal: .principal, provider: .operator.provider.principal, datacenter: .operator.datacenter.name, owner: .operator.datacenter.owner.name, city: .operator.datacenter.city, country: .operator.datacenter.country, continent: .operator.datacenter.continent }'
echo "---"

echo "provider.principal"
jq_fields .operator.provider.principal | sort | uniq -c
echo "---"

echo "datacenter.name"
jq_fields .operator.datacenter.name | sort | uniq -c
echo "---"

echo "datacenter.owner.name"
jq_fields .operator.datacenter.owner.name | sort | uniq -c
echo "---"

echo "datacenter.city"
jq_fields .operator.datacenter.city | sort | uniq -c
echo "---"

echo "datacenter.country"
jq_fields .operator.datacenter.country | sort | uniq -c
echo "---"

echo "datacenter.continent"
jq_fields .operator.datacenter.continent | sort | uniq -c
echo "---"

echo "decentralization overview"
echo "$CREATE_SUBNET_OUTPUT" | jq .score_after
echo "# Note"
echo "$CREATE_SUBNET_OUTPUT" | jq -r .comment
echo "---"

echo
echo "proposal command:"
echo "-------"
echo "./mainnet-op propose-to-create-subnet application ${SUBNET_VERSION} ${NODE_IDS[@]}"
