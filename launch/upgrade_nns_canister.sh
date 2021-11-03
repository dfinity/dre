#!/usr/bin/env bash

# This is a script that upgrades all NNS canisters specified in a canister_id.json files.
# The wasm modules are expected to be in the same directory as the canister_ids.json.
# This matches what is produced by the dfinity.rs.ic-nns-bundle-release nix derivation.

# Simplest way to test this:
# From the prod nix shell (to have dfx and jq in the path):
#
# cd ../rs/nns # This is just to have a dfx.json to make dfx happy
# dfx start --host 127.0.0.1:8080 --clean --background
#
# cd $(mktemp -d)
# curl https://blobules.dfinity.systems/dfinity-ci-build.dfinity/ic-nns-bundle/0.0.0/x86_64-linux/ic-nns-bundle-0.0.0.tar.gz --output - | tar zxf -
#
# Run ic-nns-init targetting http://localhost:8080 (no option => use test neurons)
#
# Sets the variable below as needed, then run.

NETWORK="mercury"
child_nns_url="http://[2600:c02:b002:15:5054:ffff:fe23:495c]:8080"
WASM_DIR="$(pwd)"
IC_ADMIN="/nix/store/0d7r7hsvq7irxv180hw44i847a7anp37-ic-admin/bin/ic-admin"
PROPOSER_PEM="$HOME/.config/dfx/identity/super-leader/identity.pem"
PROPOSER_NEURON_ID="49"

for canister in $(jq -r '. | keys | .[]' "$WASM_DIR/canister_ids.json"); do
    echo ""
    echo "Trying to upgrading $canister by proposal..."

    canister_id=$(jq -r ".\"${canister}\".\"${NETWORK}\"" "${WASM_DIR}/canister_ids.json")
    wasm="${WASM_DIR}/${canister}-canister.wasm"

    if [ ! -f "${wasm}" ]; then
        # The 'lifeline' canister does not follow the `-canister.wasm` convention.
        # For robustness just fallback to dropping the -canister suffix for any canister
        wasm="${WASM_DIR}/${canister}.wasm"
    fi

    if [ ! -f "${wasm}" ]; then
        echo "Could not find a wasm for ${canister}, skipping."
    fi

    echo "  canister_id=${canister_id}"
    echo "  wasm=${wasm}"

    propose_cmd="$IC_ADMIN \
	    --nns-url=${child_nns_url} \
	    --secret-key-pem=${PROPOSER_PEM} \
		propose-to-change-nns-canister \
		  ${PROPOSER_NEURON_ID} \
		  --mode=upgrade \
		  --canister-id=${canister_id} \
		  --wasm-module-path=${wasm}"

    #--test-neuron-proposer"
    # TODO(REL-55): For bootstrap we don't want test neurons. The argument `--test-neuron-proposer`
    # will have to be modified to handle a proposer's identity.

    # TODO(REL-55): If the proposer does not, through majority or through transitive following,
    # causes a proposal to be immediately accepted, then some voting command is needed here.

    # TODO(REL-55): propose_to_change_nns_canister does not work for the root.

    if ! $propose_cmd; then
        # There are many reasons for transient failure. For instance, the governance canister
        # may not yet have restarted after being upgraded. Let's sleep a bit and retry.
        sleep 10
        $propose_cmd
    fi

    # TODO(REL-55): this just verifies that the proposal was properly submitted, not that it was
    # accepted, and not that he proposal actually happened. The best way to verify is probably to
    # inject a few harmless bytes at the end of the wasm module to force a wasm module hash change,
    # and then use the root canister to observe the hash change.

done
