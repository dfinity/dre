#!/bin/bash

set -eEuo pipefail

if [  "$#" -lt 2 ]; then
  echo "Usage: $0 SUBNET|\"none\" HOSTS..."
  exit 1
fi

subnet=$1

nodes="["
for node in "${@:2}"
do
    nodes="$nodes\"$node\","
done
nodes="${nodes%,}]"

# prints out detailed information about every node to be added
curl --fail --silent https://dashboard.mercury.dfinity.systems/api/proxy/registry/nodes | jq --arg subnet "$subnet" --argjson nodes "$nodes" '.[] | select(.hostname | IN($nodes[])) | { hostname: .hostname, dirty: .dirty, principal: .principal, provider: .operator.provider.principal, datacenter: .operator.datacenter.name, owner: .operator.datacenter.owner.name, city: .operator.datacenter.city, country: .operator.datacenter.country, continent: .operator.datacenter.continent }'

echo "provider.principal"
curl --fail --silent https://dashboard.mercury.dfinity.systems/api/proxy/registry/nodes | jq --arg subnet "$subnet" --argjson nodes "$nodes" '.[] | select((.hostname | IN($nodes[])) or (.subnet // "" | startswith($subnet)) ) | .operator.provider.principal' | sort | uniq -c
echo "---"

echo "datacenter.name"
curl --fail --silent https://dashboard.mercury.dfinity.systems/api/proxy/registry/nodes | jq --arg subnet "$subnet" --argjson nodes "$nodes" '.[] | select((.hostname | IN($nodes[])) or (.subnet // "" | startswith($subnet)) ) | .operator.datacenter.name' | sort | uniq -c
echo "---"

echo "datacenter.owner.name"
curl --fail --silent https://dashboard.mercury.dfinity.systems/api/proxy/registry/nodes | jq --arg subnet "$subnet" --argjson nodes "$nodes" '.[] | select((.hostname | IN($nodes[])) or (.subnet // "" | startswith($subnet)) ) | .operator.datacenter.owner.name' | sort | uniq -c
echo "---"

echo "datacenter.city"
curl --fail --silent https://dashboard.mercury.dfinity.systems/api/proxy/registry/nodes | jq --arg subnet "$subnet" --argjson nodes "$nodes" '.[] | select((.hostname | IN($nodes[])) or (.subnet // "" | startswith($subnet)) ) | .operator.datacenter.city' | sort | uniq -c
echo "---"

echo "datacenter.country"
curl --fail --silent https://dashboard.mercury.dfinity.systems/api/proxy/registry/nodes | jq --arg subnet "$subnet" --argjson nodes "$nodes" '.[] | select((.hostname | IN($nodes[])) or (.subnet // "" | startswith($subnet)) ) | .operator.datacenter.country' | sort | uniq -c
echo "---"

echo "datacenter.continent"
curl --fail --silent https://dashboard.mercury.dfinity.systems/api/proxy/registry/nodes | jq --arg subnet "$subnet" --argjson nodes "$nodes" '.[] | select((.hostname | IN($nodes[])) or (.subnet // "" | startswith($subnet)) ) | .operator.datacenter.continent' | sort | uniq -c

if [ "$subnet" == "none" ]; then
    echo "proposal command:"
    echo "-------"
    echo -n ./mainnet-op  propose-to-create-subnet $(($(curl --fail --silent https://dashboard.mercury.dfinity.systems/api/proxy/registry/subnets | jq -r --arg subnet "$subnet" '.[] | .metadata.name | sub("^App "; "")' | sort -n -r | head -n 1) + 1)) application 32d4e9c61c8b284d1bebed290df8d9b2efad2fc6
    echo -n " "
    curl --fail --silent https://dashboard.mercury.dfinity.systems/api/proxy/registry/nodes | jq --arg subnet "$subnet" --argjson nodes "$nodes" '.[] | select(.hostname | IN($nodes[])) | .principal' | xargs
else
    echo "proposal command:"
    echo "-------"
    echo -n ./mainnet-op propose-to-add-nodes-to-subnet $(curl --fail --silent https://dashboard.mercury.dfinity.systems/api/proxy/registry/subnets | jq -r --arg subnet "$subnet" '.[] | select(.principal | startswith($subnet)) | .metadata.name | sub("^App "; "")')
    echo -n " "
    curl --fail --silent https://dashboard.mercury.dfinity.systems/api/proxy/registry/nodes | jq --arg subnet "$subnet" --argjson nodes "$nodes" '.[] | select(.hostname | IN($nodes[])) | .principal' | xargs
fi
