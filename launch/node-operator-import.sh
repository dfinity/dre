#!/usr/bin/env bash

if [[ "${1:-}" == "--help" ]]; then
    cat <<-EOF
	Usage:
	* Download the node allowance sheet as tab separated values (.tsv).
	* $0 <NETWORK>
	EOF
    exit 0
fi
set -euxo pipefail

network="${1}"
tab_separated_spreadsheet="../testnet/env/$network/verified_keys.tsv"

# Write a new key listing:
{
    ls "../testnet/env/$network/data_centers" | grep -E '[.]der$' | jq -R '{(.|sub(".der$"; "")):{node_allowance: (0), node_provider: "OPERATOR KEY MISSING FROM SPREADSHEET" }}'
    if test -n "$tab_separated_spreadsheet"; then
        sed 1d "$tab_separated_spreadsheet" | jq -R 'split("\t") | select( (.[5]|sub("[[:space:]]+"; "")|sub(".der$"; "")) != "" ) | 
        {(.[5]|sub("[[:space:]]+"; "")|sub(".der$"; "")): {node_allowance: (.[4]|tonumber), node_provider: ("provider_keys/\(.[6])")}}'
    fi
} | jq -s add >"../testnet/env/$network/data_centers/meta.json"

{
    ls "../testnet/env/$network/data_centers/provider_keys" | grep -E '[.]der$' | jq -R '{(.|sub(".der$"; "")):{node_provider: "PROVIDER KEY MISSING FROM SPREADSHEET!!!!!!!" }}'
    if test -n "$tab_separated_spreadsheet"; then
        sed 1d "$tab_separated_spreadsheet" | jq -R 'split("\t") | select( (.[6]|sub("[[:space:]]+"; "")|sub(".der$"; "")) != "" ) |
        {(.[6]|sub("[[:space:]]+"; "")|sub(".der$"; "")): {node_provider: ("PRESENT")}}'
    fi
} | jq -s add >"../testnet/env/$network/data_centers/provider_keys/.sanity-check.json"

(
    set -euo pipefail
    cd "../testnet/env/$network/data_centers/"
    jq -r '. | keys[]' meta.json | while read line; do ls | grep -E "^${line}.der$" >/dev/null || echo "MISSING KEY: $line.der"; done
)
