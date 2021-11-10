error() {
    print_red "ERROR: $1"
    exit 1
}

echostderr() { echo "$@" 1>&2; }

print_param() {
    echo "    $1: $2"
}

print_usage() {
    echo >&2 "USAGE: $0 <operation-name> [...]"
    echo >&2 "  supported operations: "
    echo >&2 "    propose-to-add-nodes-to-subnet"
    echo >&2 "    propose-to-create-subnet"
    echo >&2 "    propose-to-remove-nodes-from-subnet"
    echo >&2 "    propose-to-remove-nodes"
    echo >&2 "    propose-to-update-subnet"
    echo >&2 "    propose-to-update-subnet-replica-version"
    echo >&2 "    propose-to-bless-replica-version"
    echo >&2 "    query"
    echo >&2 "    query-subnet-versions"
}

print_red() {
    echo -e "\033[0;31m$*\033[0m" 1>&2
}

print_green() {
    echo -e "\033[0;32m$*\033[0m"
}

print_blue() {
    echo -e "\033[0;34m$*\033[0m"
}

do_you_want_to_continue() {
    echo ""
    read -r -p "Do you want to continue? [y/N] " response
    if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
        print_green "continuing..."
    else
        print_red "aborting..."
        exit 1
    fi
}

REPO_ROOT=$(
    cd "$(dirname "$0")"
    git rev-parse --show-toplevel
)
cd "$REPO_ROOT"
REPO_BIN="$REPO_ROOT/bash-ops/bin"
mkdir -p "$REPO_BIN"
export PATH="$REPO_BIN:$PATH"

refresh_ic_submodule() {
    if ! [ -r "$REPO_ROOT"/ic/gitlab-ci/src/artifacts/newest_sha_with_disk_image.sh ]; then
        git submodule update --init --recursive --remote
        git submodule foreach -q git remote add github git@github.com:dfinity-lab/dfinity.git
        git submodule foreach -q git fetch --all
    fi
}

install_ic_admin() {
    refresh_ic_submodule
    GIT_REVISION=$(
        cd "$REPO_ROOT"/ic
        gitlab-ci/src/artifacts/newest_sha_with_disk_image.sh "origin/post-merge-tests-passed"
    )
    if [ -n "$GIT_REVISION" ]; then
        if [ "$(uname)" == "Darwin" ]; then
            curl https://download.dfinity.systems/blessed/ic/$GIT_REVISION/nix-release/x86_64-darwin/ic-admin.gz -o - | gunzip -c >|"$REPO_BIN"/ic-admin.$GIT_REVISION
        else
            curl https://download.dfinity.systems/blessed/ic/$GIT_REVISION/release/ic-admin.gz -o - | gunzip -c >|"$REPO_BIN"/ic-admin.$GIT_REVISION
        fi
        chmod +x "$REPO_BIN"/ic-admin.$GIT_REVISION
        ln -sf "$REPO_BIN"/ic-admin.$GIT_REVISION "$REPO_BIN"/ic-admin
    fi
    touch "$REPO_BIN/ic-admin.updated"
}

check_ic_admin() {
    if [[ -z "$(find "$REPO_BIN" -mtime -24 -name "ic-admin.*")" ]]; then
        echo "ic-admin executable is missing or older than 24h, trying to update"
        install_ic_admin
    fi
    if [[ ! -x $REPO_BIN/ic-admin ]]; then
        error "ic-admin executable still not found, exiting"
    fi
}

install_jq() {
    if [ "$(uname)" == "Darwin" ]; then
        curl -L https://github.com/stedolan/jq/releases/download/jq-1.6/jq-osx-amd64 -o "$REPO_BIN"/jq
    else
        curl -L https://github.com/stedolan/jq/releases/download/jq-1.6/jq-linux64 -o "$REPO_BIN"/jq
    fi
    chmod +x "$REPO_BIN"/jq
}

check_jq() {
    if [[ ! -x $REPO_BIN/jq ]]; then
        echo "jq executable not found, trying to install"
        install_jq
    fi
    if [[ ! -x $REPO_BIN/jq ]]; then
        error "jq executable still not found, exiting"
    fi
}

check_env_common_for_proposal() {
    if [ -z "$PROPOSER_NEURON_INDEX" ]; then
        error "PROPOSER_NEURON_INDEX is not set"
    fi
    if [ -z "$NNS_URL" ]; then
        error "NNS_URL is not set"
    fi
}

print_env_common_for_proposal() {
    print_env
    print_param NETWORK "$NETWORK"
    print_param NNS_URL "$NNS_URL"
    print_param PROPOSER_NEURON_INDEX "$PROPOSER_NEURON_INDEX"
    print_param PROPOSAL_TITLE "$PROPOSAL_TITLE"
    if [[ -n "${PROPOSAL_URL:-}" ]]; then
        print_param PROPOSAL_URL "$PROPOSAL_URL"
    fi
    if [[ -n "${PROPOSAL_SUMMARY_FILE:-}" ]]; then
        echo "=== PROPOSAL_SUMMARY_FILE $PROPOSAL_SUMMARY_FILE contents START ====="
        cat $PROPOSAL_SUMMARY_FILE
        echo "=== PROPOSAL_SUMMARY_FILE $PROPOSAL_SUMMARY_FILE contents END   ====="
    fi
}

check_env_mainnet() {
    if [ -z "$HSM_SLOT" ]; then
        error "HSM_SLOT is not set"
    fi
    if [ -z "$KEY_ID" ]; then
        error "KEY_ID is not set"
    fi
    if [ ! -f "$HSM_PIN_FILE" ]; then
        error "Required file $HSM_PIN_FILE is missing"
    fi
}

print_env() {
    print_param ic-admin "$IC_ADMIN"
    if [[ $NETWORK == "mainnet" ]]; then
        print_param HSM_SLOT "$HSM_SLOT"
        print_param KEY_ID "$KEY_ID"
    else
        print_param PROPOSER_NAME "$PROPOSER_NAME"
    fi
}

get_testnet_nns_url() {
    git_root="$(git rev-parse --show-toplevel)"
    $git_root/ic/testnet/env/$NETWORK/hosts --nodes | head -n1 | cut -d' ' -f2
}

check_env_testnet() {
    if [ -z "$PROPOSER_NAME" ]; then
        error "PROPOSER_NAME is not set"
    fi
    if [ ! -f "$TESTNET_IDENTITY_PEM_FILE" ]; then
        error "Required file $TESTNET_IDENTITY_PEM_FILE is missing"
    fi
}

check_version_commit_is_blessed() {
    version_commit="$1"
    # check version exists
    print_green Checking that version $version_commit is blessed...
    $IC_ADMIN --nns-url $NNS_URL get-blessed-replica-versions | grep -q "$version_commit"
    if [ $? != 0 ]; then
        error "Replica version $version_commit is not blessed."
    fi
}

get_new_subnet_params() {
    echo --ingress-bytes-per-block-soft-cap 2097152 \
        --max-ingress-bytes-per-message 2097152 \
        --max-ingress-messages-per-block 1000 \
        --unit-delay-millis 1000 \
        --initial-notary-delay-millis 600 \
        --dkg-interval-length 499 \
        --gossip-max-artifact-streams-per-peer 20 \
        --gossip-max-chunk-wait-ms 15000 \
        --gossip-max-duplicity 1 \
        --gossip-max-chunk-size 4096 \
        --gossip-receive-check-cache-size 5000 \
        --gossip-pfn-evaluation-period-ms 3000 \
        --gossip-registry-poll-period-ms 3000 \
        --gossip-retransmission-request-ms 60000 \
        --max-block-payload-size 0
}

get_subnet_id_from_index() {
    subnet_index="$1"

    # get subnet id from subnet list
    subnet_id=$($IC_ADMIN --nns-url "$NNS_URL" get-subnet-list | jq -r ".[$subnet_index]")

    # double-check that subnet index and the subnet id match
    $IC_ADMIN --nns-url "$NNS_URL" get-subnet $subnet_index | grep -q "$subnet_id"
    if [ $? != 0 ]; then
        error "Script error: failed to map subnet index $subnet_index to subnet id (got $subnet_id)"
    fi

    echo $subnet_id
}

maybe_propose_to_remove_nodes_from_subnet() {
    if [ $# -lt 1 ]; then
        print_red "Node ids not specified"
        echo >&2 "USAGE: $0 $OP_NAME <node-id> [<node-id> ...]"
        exit 1
    fi
    nodes_to_remove=$@
    PROPOSAL_TITLE=${PROPOSAL_TITLE:-"Remove node(s) from subnet(s)"}
    echo
    echo "Please provide the MOTIVATION for this proposal and ctrl-d when done"
    MOTIVATION="${MOTIVATION:-$(cat)}"
    PROPOSAL_SUMMARY_FILE=$(mktemp)
    cat >$PROPOSAL_SUMMARY_FILE <<_EOF
Remove node(s) $nodes_to_remove from their subnet(s), which makes them unassigned.

Motivation: $MOTIVATION
_EOF
    print_env_common_for_proposal

    cmd=($IC_ADMIN $AUTH_PARAMS --nns-url="$NNS_URL"
        propose-to-remove-nodes-from-subnet
        --proposer $PROPOSER_NEURON_INDEX
        --proposal-title="$PROPOSAL_TITLE"
        --summary-file "$PROPOSAL_SUMMARY_FILE"
        $nodes_to_remove)

    # print the command before executing it
    printf '%q ' "${cmd[@]}"
    echo

    do_you_want_to_continue

    set -x
    "${cmd[@]}"
    rm "$PROPOSAL_SUMMARY_FILE"
}

maybe_propose_to_remove_nodes() {
    if [ $# -lt 1 ]; then
        print_red "Node ids not specified"
        echo >&2 "USAGE: $0 $OP_NAME <node-id> [<node-id> ...]"
        exit 1
    fi
    nodes_to_remove=$@
    PROPOSAL_TITLE=${PROPOSAL_TITLE:-"Remove node(s) from the Registry."}
    echo
    echo "Please provide the MOTIVATION for this proposal and ctrl-d when done"
    MOTIVATION="${MOTIVATION:-$(cat)}"
    PROPOSAL_SUMMARY_FILE=$(mktemp)
    cat >$PROPOSAL_SUMMARY_FILE <<_EOF
Remove node(s) $nodes_to_remove from the Registry.

Motivation: $MOTIVATION
_EOF
    print_env_common_for_proposal

    cmd=($IC_ADMIN $AUTH_PARAMS --nns-url="$NNS_URL"
        propose-to-remove-nodes
        --proposer $PROPOSER_NEURON_INDEX
        --proposal-title="$PROPOSAL_TITLE"
        --summary-file "$PROPOSAL_SUMMARY_FILE"
        $nodes_to_remove)

    # print the command before executing it
    printf '%q ' "${cmd[@]}"
    echo

    do_you_want_to_continue

    set -x
    "${cmd[@]}"
    rm "$PROPOSAL_SUMMARY_FILE"
}

maybe_propose_to_add_nodes_to_subnet() {
    if [ $# -lt 2 ]; then
        print_red "Too few arguments"
        echo >&2 "USAGE: $0 $OP_NAME <subnet-number> <node-id> [<node-id> ...]"
        exit 1
    fi
    subnet_index=$1
    subnet_id=$(get_subnet_id_from_index "$subnet_index")
    subnet_id_short=$(echo $subnet_id | cut -d- -f1)
    shift
    nodes_to_add=$@
    PROPOSAL_TITLE=${PROPOSAL_TITLE:-"Add node(s) to subnet $subnet_id_short"}
    echo
    echo "Please provide the MOTIVATION for this proposal and ctrl-d when done"
    MOTIVATION="${MOTIVATION:-$(cat)}"
    PROPOSAL_SUMMARY_FILE=$(mktemp)
    cat >$PROPOSAL_SUMMARY_FILE <<_EOF
Add node(s) $nodes_to_add to subnet $subnet_id

Motivation: $MOTIVATION
_EOF
    print_green "Proposing on [$NETWORK] to add nodes to subnet [$subnet_id_short] with the following params:"
    print_env_common_for_proposal
    print_param "nodes to be add to the subnet" "$nodes_to_add"

    cmd=($IC_ADMIN $AUTH_PARAMS --nns-url="$NNS_URL"
        propose-to-add-nodes-to-subnet
        --subnet "$subnet_id"
        --proposer $PROPOSER_NEURON_INDEX
        --proposal-title "$PROPOSAL_TITLE"
        --summary-file "$PROPOSAL_SUMMARY_FILE"
        $nodes_to_add)

    # print the command before executing it
    printf '%q ' "${cmd[@]}"
    echo

    do_you_want_to_continue

    set -x
    "${cmd[@]}"
    rm "$PROPOSAL_SUMMARY_FILE"
}

maybe_propose_to_create_subnet() {
    if [ $# -lt 3 ]; then
        print_red "Too few arguments"
        echo >&2 "USAGE: $0 $OP_NAME <subnet-number> <subnet-type> <version-commit>"
        exit 1
    fi
    subnet_number="$1"
    subnet_type="$2"
    version_commit="$3"
    shift
    shift
    shift
    node_ids=$@

    new_subnet_params="${NEW_SUBNET_PARAMS:-$(get_new_subnet_params)}"

    case "$subnet_type" in
        application) : ;;
        verified_application) : ;;
        *)
            error "Unsupported application type: $subnet_type (supported types: application, verified_application)"
            exit 1
            ;;
    esac

    set +euo pipefail
    check_version_commit_is_blessed "$version_commit"

    PROPOSAL_TITLE=${PROPOSAL_TITLE:-"Create new ${subnet_type} subnet"}
    echo
    echo "Please provide the MOTIVATION for this proposal and ctrl-d when done"
    MOTIVATION="${MOTIVATION:-$(cat)}"
    PROPOSAL_SUMMARY_FILE=$(mktemp)
    cat >$PROPOSAL_SUMMARY_FILE <<_EOF
Create new ${subnet_type} subnet with replica revision ${version_commit}

Motivation: $MOTIVATION
_EOF
    print_env_common_for_proposal
    print_param "app subnet number" "$subnet_number"
    print_param "version commit" "$version_commit"
    print_param "nodes for the new subnet" "$node_ids"
    print_param "new subnet params" "$new_subnet_params"

    cmd=($IC_ADMIN $AUTH_PARAMS --nns-url="$NNS_URL"
        propose-to-create-subnet
        --proposer $PROPOSER_NEURON_INDEX
        --subnet-handler-id "unused"
        $new_subnet_params
        --subnet-type "${subnet_type}"
        --replica-version-id "${version_commit}"
        --proposal-title "$PROPOSAL_TITLE"
        --summary-file "$PROPOSAL_SUMMARY_FILE"
        $node_ids)

    # print the command before executing it
    printf '%q ' "${cmd[@]}"
    echo

    do_you_want_to_continue

    set -x
    "${cmd[@]}"
    rm "$PROPOSAL_SUMMARY_FILE"
}

maybe_propose_to_bless_replica_version() {
    if [ ! $# == 1 ]; then
        echo >&2 "USAGE: $0 $OP_NAME <version-commit>"
        exit 1
    fi
    version_commit=$1

    if ! command -v debugfs &>/dev/null; then
        error "debugfs not found, please install it (e2fsprogs-package), and add it to your PATH"
    fi
    if ! command -v sha256sum &>/dev/null; then
        error "sha256sum not found, please install it (coreutils-package), and add it to your PATH"
    fi

    ALL_ARTEFACTS_DIR="${ALL_ARTEFACTS_DIR:-$HOME/all-artefacts}"

    set -euo pipefail
    download_dir="$ALL_ARTEFACTS_DIR/$version_commit"
    print_green Downloading artefacts for commit $version_commit to directory $download_dir/
    mkdir -p "$download_dir"
    git_root="$(git rev-parse --show-toplevel)"
    "$git_root"/ic/gitlab-ci/src/artifacts/rclone_download.py --git-rev "$version_commit" --remote-path guest-os/update-img --out "$download_dir/guest-os/update-img" --blessed-only

    SHA256="$(sha256sum "${download_dir}/guest-os/update-img/update-img.tar.gz" | cut -f 1 -d ' ')"
    UPDATE_URL="https://download.dfinity.systems/blessed/ic/${version_commit}/guest-os/update-img/update-img.tar.gz"

    verification_dir="$ALL_ARTEFACTS_DIR/update-verification"
    rm -fr "$verification_dir"
    mkdir -p "$verification_dir"
    verification_img="$verification_dir/update-img.tar.gz"

    print_green Downloading the update image from $UPDATE_URL to $verification_img
    curl --fail "$UPDATE_URL" >"${verification_img}"
    download_sha="$(sha256sum "${verification_img}" | cut -f 1 -d ' ')"
    if [ ! "$download_sha" == "$SHA256" ]; then
        error "Download sha did not match! ($SHA256 vs. $download_sha)" >&2
    fi
    print_green "SHA256-hashes of update-img.tar.gz do match ($SHA256)"

    print_green "Checking the version in the image..."
    # Check version in image
    OUT="${verification_dir}/image_version_check"
    rm -fr "$OUT"
    mkdir -p "$OUT"
    cd "$OUT"
    ls -anh "$verification_img"
    tar -tf "$verification_img"
    tar -xf "$verification_img"
    # Check current file number in image
    version_in_img="$(echo "cat /opt/ic/share/version.txt" | debugfs root.img -f - 2>/dev/null)"
    if [[ ! "${version_in_img}" == *"$version_commit"* ]]; then
        error "Version in root.img ($version_in_img) does not match upgrade commit ($version_commit)" >&2
    fi
    print_green "Version in the image matches $version_commit"

    PROPOSAL_TITLE=${PROPOSAL_TITLE:-"Elect/Bless new replica binary revision (commit $version_commit)"}
    echo
    echo "Please provide the Changelog (as bullets, without the title) for this release and ctrl-d when done"
    CHANGELOG="${CHANGELOG:-$(cat)}"
    PROPOSAL_SUMMARY_FILE=$(mktemp)
    cat >$PROPOSAL_SUMMARY_FILE <<_EOF
Elect/Bless new replica binary revision (commit $version_commit)

Changelog:
$CHANGELOG
_EOF
    print_env_common_for_proposal
    print_param "replica version" "$version_commit"
    print_param "release package URL" "$UPDATE_URL"
    print_param "release package SHA256" "$SHA256"

    cmd=($IC_ADMIN $AUTH_PARAMS --nns-url="$NNS_URL"
        propose-to-bless-replica-version-flexible
        --proposal-title "$PROPOSAL_TITLE"
        --summary-file $PROPOSAL_SUMMARY_FILE
        "$version_commit"
        ignored_replica_url ignored_replica_sha256
        ignored_node_manager_url ignored_node_manager_sha256
        "$UPDATE_URL" "$SHA256"
        --proposer $PROPOSER_NEURON_INDEX)

    # print the command before executing it
    printf '%q ' "${cmd[@]}"
    echo

    do_you_want_to_continue

    set -x
    "${cmd[@]}"
    rm "$PROPOSAL_SUMMARY_FILE"
}

maybe_propose_to_update_subnet_replica_version() {
    if [ ! $# == 2 ]; then
        echo >&2 "USAGE: $0 $OP_NAME <subnet-index> <version-commit>"
        exit 1
    fi
    subnet_index="$1"
    version_commit="$2"

    set +euo pipefail

    check_version_commit_is_blessed "$version_commit"

    # get subnet id, to make sure the right subnet gets upgraded
    subnet_id=$(get_subnet_id_from_index "$subnet_index")

    set -euo pipefail
    PROPOSAL_TITLE=${PROPOSAL_TITLE:-"Update subnet $subnet_id to replica version $version_commit"}
    echo
    echo "Please provide the MOTIVATION for this proposal and ctrl-d when done"
    MOTIVATION="${MOTIVATION:-$(cat)}"
    PROPOSAL_SUMMARY_FILE=$(mktemp)
    cat >$PROPOSAL_SUMMARY_FILE <<_EOF
Update subnet $subnet_id to replica version $version_commit

Motivation: $MOTIVATION
_EOF
    print_env_common_for_proposal

    cmd=($IC_ADMIN $AUTH_PARAMS --nns-url="$NNS_URL"
        propose-to-update-subnet-replica-version
        --proposal-title "$PROPOSAL_TITLE"
        --summary-file "$PROPOSAL_SUMMARY_FILE"
        $subnet_index $version_commit
        --proposer $PROPOSER_NEURON_INDEX)

    # print the command before executing it
    printf '%q ' "${cmd[@]}"
    echo

    do_you_want_to_continue

    set -x
    "${cmd[@]}"
    rm "$PROPOSAL_SUMMARY_FILE"
}

maybe_propose_to_update_subnet() {
    if [ $# -lt 2 ]; then
        echo >&2 "USAGE: $0 $OP_NAME <subnet-index> <param> [<param>...]"
        echo >&2 "    available params are OPTIONS listed by 'ic-admin propose-to-update-subnet --help':"
        $IC_ADMIN propose-to-update-subnet --help
        exit 1
    fi

    subnet_index="$1"
    shift
    params="$@"

    # get subnet id, to make sure the right subnet gets updated
    subnet_id=$(get_subnet_id_from_index "$subnet_index")

    set -euo pipefail
    PROPOSAL_TITLE=${PROPOSAL_TITLE:-"Update parameters of subnet $subnet_id"}
    echo
    echo "Please provide the MOTIVATION for this proposal and ctrl-d when done"
    MOTIVATION="${MOTIVATION:-$(cat)}"
    PROPOSAL_SUMMARY_FILE=$(mktemp)
    cat >$PROPOSAL_SUMMARY_FILE <<_EOF
Update parameters of subnet $subnet_id: $params

Motivation: $MOTIVATION
_EOF
    print_env_common_for_proposal

    cmd=($IC_ADMIN $AUTH_PARAMS --nns-url="$NNS_URL"
        propose-to-update-subnet
        --proposal-title "$PROPOSAL_TITLE"
        --summary-file "$PROPOSAL_SUMMARY_FILE"
        $params
        --subnet "$subnet_id"
        --proposer $PROPOSER_NEURON_INDEX)

    # print the command before executing it
    printf '%q ' "${cmd[@]}"
    echo

    do_you_want_to_continue

    set -x
    "${cmd[@]}"
    rm "$PROPOSAL_SUMMARY_FILE"
}

maybe_query() {
    if [ $# -lt 1 ]; then
        echo >&2 "USAGE: $0 $OP_NAME <selector> [params]"
        echo >&2 "  where <selector> is one of the following, (and params depend on <selector>):"
        ic-admin --help | grep get- >&2
        exit 1
    fi

    ic_admin_args=$@
    $IC_ADMIN --nns-url="$NNS_URL" $ic_admin_args
}

query_subnet_versions() {
    ic_admin_args=$@
    set -x
    $IC_ADMIN --nns-url="$NNS_URL" $ic_admin_args get-topology | jq -r '.topology.subnets | to_entries[] | "\(.key)\t\(.value.records[] | .value.replica_version_id)\t\(.value.records[] | .value.subnet_type)"' | awk '{ print NR-1, $0 }'
}

maybe_do_operation() {
    case $OP_NAME in
        propose-to-add-nodes-to-subnet)
            maybe_propose_to_add_nodes_to_subnet $OP_ARGS
            ;;
        propose-to-create-subnet)
            maybe_propose_to_create_subnet $OP_ARGS
            ;;
        propose-to-remove-nodes-from-subnet)
            maybe_propose_to_remove_nodes_from_subnet $OP_ARGS
            ;;
        propose-to-remove-nodes)
            maybe_propose_to_remove_nodes $OP_ARGS
            ;;
        propose-to-update-subnet-replica-version)
            maybe_propose_to_update_subnet_replica_version $OP_ARGS
            ;;
        propose-to-update-subnet)
            maybe_propose_to_update_subnet $OP_ARGS
            ;;
        propose-to-bless-replica-version)
            maybe_propose_to_bless_replica_version $OP_ARGS
            ;;
        query)
            maybe_query $OP_ARGS
            ;;
        query-subnet-versions)
            query_subnet_versions $OP_ARGS
            ;;
        *)
            error "Unknown operation [$OP_NAME]"
            ;;
    esac
}
