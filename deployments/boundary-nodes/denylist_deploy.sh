#!/usr/bin/env bash

# Re-deploy the denylist to boundary node VMs
#
# This script takes one positional argument:
#   <deployment_identifier>: The deployment referenced in `deployments/boundary-nodes/env/${deployment}`

set -eEuo pipefail

cd "$(dirname "$0")"
REPO_ROOT="$(git rev-parse --show-toplevel)"

function exit_usage() {
    if (($# < 1)); then
        echo >&2 "Usage: denylist_deploy.sh <deployment_name> [--denylist <path_to_denylist.map>]"
        echo >&2 "    --denylist <path> Specify a denylist of canisters for the Boundary Nodes\n"
        exit 1
    fi
}

DENY_LIST=""
DEPLOYMENT=""

while [ $# -gt 0 ]; do
    case "${1}" in
        --denylist)
            DENY_LIST="${2:-}"
            if [[ -z "${DENY_LIST}" ]]; then
                echo "DENY_LIST file not set"
                exit_usage
            fi
            shift
            ;;
        -?*) exit_usage ;;
        *) DEPLOYMENT="$1" ;;
    esac
    shift
done

if [[ -z "$DEPLOYMENT" ]]; then
    echo "ERROR: No deployment specified."
    exit_usage
fi

if [[ -z "$DENY_LIST" ]]; then
    echo "ERROR: No denylist specified."
    exit_usage
fi

INVENTORY="$REPO_ROOT/deployments/boundary-nodes/env/${DEPLOYMENT}/hosts"
SSH_ARGS="-q -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no"
BOUNDARY_IPS=$($INVENTORY --media-json | jq -r '.datacenters | map(.boundary_nodes[0].ipv6_address) | map(split("/")[0]) | .[]')

task() {
    echo "Procesing ${1}"
    cat ${DENY_LIST} | ssh ${SSH_ARGS} "admin@${1}" 'cat - >/tmp/denylist.map && (sudo umount /etc/nginx/denylist.map || true) && sudo mount --bind /tmp/denylist.map /etc/nginx/denylist.map && sudo systemctl reload nginx'
    echo "Finished ${1}"
}
PIDS=()
for IP in $BOUNDARY_IPS; do
    task "${IP}" &
    PIDS+=("$!")
done

sleep 1
echo "waiting on ${#PIDS[@]} tasks"
wait ${PIDS[@]}
