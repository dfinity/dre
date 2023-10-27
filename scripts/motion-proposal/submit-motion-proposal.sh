#!/bin/bash

set -eEuo pipefail
BASEDIR=$(
    cd "$(dirname "$0")"
    pwd
)
source "$BASEDIR"/../lib.sh
cd "$BASEDIR"

if [[ -z "${1:-}" ]]; then
    echo "Please provide the summary file as the first argument"
    exit 1
fi

SUMMARY=$(cat "$BASEDIR/$1" | sed 's/"/\\"/g')

TITLE=$(echo "$SUMMARY" | head -n1 | sed 's/^##*  *//g')
MOTION_TEXT=$TITLE

echo "!!! Dry run"
echo "!!! ***************************************************************************"
echo "!!! * NOTE: invalid proposer is intentional, please ignore the submission error"
echo "!!! ***************************************************************************"
echo

set -x
dfx --identity proposals canister --network ic call rrkah-fqaaa-aaaaa-aaaaq-cai manage_neuron \
    "(record {id = null; command=opt variant {MakeProposal=record {url=\"\"; title=\"$TITLE\";action=opt variant {Motion=record {motion_text=\"$MOTION_TEXT\"}}; summary=\"$SUMMARY\"}}; neuron_id_or_subaccount=opt variant {NeuronId=record {id=$PROPOSER_NEURON_INDEX:nat64}}})"
set +x

do_you_want_to_continue

export DFX_HSM_PIN=$(load_hsm_pin) || exit $?
dfx --identity hsm canister --network ic call rrkah-fqaaa-aaaaa-aaaaq-cai manage_neuron \
    "(record {id = null; command=opt variant {MakeProposal=record {url=\"\"; title=\"$TITLE\";action=opt variant {Motion=record {motion_text=\"$MOTION_TEXT\"}}; summary=\"$SUMMARY\"}}; neuron_id_or_subaccount=opt variant {NeuronId=record {id=$PROPOSER_NEURON_INDEX:nat64}}})"
