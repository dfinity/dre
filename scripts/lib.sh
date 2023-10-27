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
# The following command works even with git worktrees: it finds the absolute path of the "main" (default) repo
# and appends the "/ic" subdirectory to it.
# This allows us to use git worktrees with a clone of the "ic" submodule only in the main worktree.
IC_REPO_ROOT="$(git rev-parse --show-superproject-working-tree --show-toplevel | head -n1)/ic"
cd "$REPO_ROOT"
REPO_BIN="$REPO_ROOT/scripts/bin"
mkdir -p "$REPO_BIN"
export PATH="$REPO_BIN:$PATH"

get_nns_subnet_git_revision() {
    $IC_ADMIN --nns-url $NNS_URL get-subnet 0 | jq -e -r '.records[0].value.replica_version_id'
}

install_ic_admin() {
    GIT_REVISION=$1
    IC_ADMIN_BIN="$REPO_BIN/ic-admin.$GIT_REVISION"
    if [[ ! -s "$IC_ADMIN_BIN" ]] || [[ ! -x "$IC_ADMIN_BIN" ]]; then # file empty or not executable
        if [ "$(uname)" == "Darwin" ]; then
            curl --silent https://download.dfinity.systems/ic/$GIT_REVISION/binaries/x86_64-darwin/ic-admin.gz -o - | gunzip -c >|"$IC_ADMIN_BIN"
        else
            curl --silent https://download.dfinity.systems/ic/$GIT_REVISION/release/ic-admin.gz -o - | gunzip -c >|"$IC_ADMIN_BIN"
        fi
        chmod +x "$IC_ADMIN_BIN"
    fi
    ln -sf "$IC_ADMIN_BIN" "$REPO_BIN/ic-admin"
}

check_ic_admin() {
    GIT_REVISION=${GIT_REVISION:-$(get_nns_subnet_git_revision)}
    IC_ADMIN_BIN="$REPO_BIN/ic-admin.$GIT_REVISION"
    if [[ ! -x "$REPO_BIN/ic-admin" ]] || [[ ! -x "$IC_ADMIN_BIN" ]] || [[ "$(readlink "$REPO_BIN/ic-admin")" != "$IC_ADMIN_BIN" ]]; then
        echo >&2 "Updating ic-admin executable to git rev. $GIT_REVISION"
        install_ic_admin "$GIT_REVISION"
    fi
    if [[ ! -x "$REPO_BIN"/ic-admin ]]; then
        error "ic-admin executable could not be downloaded and installed, exiting"
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
    if [[ ! -x "$REPO_BIN"/jq ]]; then
        echo >&2 "jq executable not found, trying to install"
        install_jq
    fi
    if [[ ! -x "$REPO_BIN"/jq ]]; then
        error "jq executable still not found, exiting"
    fi
}

check_env_common_for_proposal() {
    if [ -z "${PROPOSER_NEURON_INDEX:-}" ]; then
        error "PROPOSER_NEURON_INDEX is not set"
    fi
    if [ -z "${NNS_URL:-}" ]; then
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

load_hsm_key_id() {
    local keyid=${HSM_KEY_ID:-${KEY_ID:-}}
    if test -z "$keyid"; then
        echo >&2 "Error: you must define variable HSM_KEY_ID or KEY_ID as per https://www.notion.so/Replica-version-blessing-proposal-10053efba4b6437684f590d540a929e1#98b4d3d31d4549e3bcd45f2877e2f654"
        exit 4
    fi
    echo "$keyid"
}

load_hsm_slot() {
    if test -z "${HSM_SLOT:-}"; then
        echo >&2 "Error: you must define variable HSM_SLOT as per https://www.notion.so/Replica-version-blessing-proposal-10053efba4b6437684f590d540a929e1#98b4d3d31d4549e3bcd45f2877e2f654"
        exit 4
    fi
    echo "$HSM_SLOT"
}

load_hsm_pin() {
    # User set the variable already.
    if [ -n "${DFX_HSM_PIN:-}" ]; then
        echo "$DFX_HSM_PIN"
        return 0
    fi

    local LEGACY_HSM_PIN_FILE="$HOME/.hsm-pin"
    if [ -f "$LEGACY_HSM_PIN_FILE" ]; then
        local DFX_HSM_PIN=$(cat "$LEGACY_HSM_PIN_FILE")
        if [ "$DFX_HSM_PIN" = "" ]; then
            echo >&2 "A nonempty PIN is required, but the PIN from $LEGACY_HSM_PIN_FILE is empty; please delete the file and retry"
            return 4
        fi
    else
        if [ -z "${HSM_PIN_FILE:-}" ]; then
            if uname -o | grep -q Linux; then
                local HSM_PIN_FILE=/run/user/$UID/hsm-pin
            else
                local HSM_PIN_FILE="$TMPDIR"/hsm-pin
            fi
        fi
        if [ -f "$HSM_PIN_FILE" ]; then
            local DFX_HSM_PIN=$(cat "$HSM_PIN_FILE")
            if [ "$DFX_HSM_PIN" = "" ]; then
                echo >&2 "A nonempty PIN is required, but the PIN from $HSM_PIN_FILE is empty; please delete the file and retry"
                return 4
            fi
        else
            read -s -p "Enter your DFX HSM PIN (to be stored in $HSM_PIN_FILE): " DFX_HSM_PIN
            if [ "$DFX_HSM_PIN" = "" ]; then
                echo >&2 "A nonempty PIN is required"
                return 4
            fi
            echo "$DFX_HSM_PIN" >"$HSM_PIN_FILE"
        fi
    fi
    echo "$DFX_HSM_PIN"
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

get_subnet_id_from_index() {
    subnet_index="$1"

    # get subnet id from subnet list
    subnet_id=$($IC_ADMIN --nns-url "$NNS_URL" get-subnet-list | jq -e -r ".[$subnet_index]")

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
    "${cmd[@]}" --dry-run
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
    "${cmd[@]}" --dry-run
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
    "${cmd[@]}" --dry-run
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
    subnet_type="$1"
    version_commit="$2"
    shift
    shift
    node_ids=$@

    case "$subnet_type" in
        application) : ;;
        verified_application) : ;;
        system) : ;;
        *)
            error "Unsupported subnet type: $subnet_type (supported types: application, verified_application, system)"
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
    print_param "version commit" "$version_commit"
    print_param "nodes for the new subnet" "$node_ids"

    cmd=($IC_ADMIN $AUTH_PARAMS --nns-url="$NNS_URL"
        propose-to-create-subnet
        --proposer $PROPOSER_NEURON_INDEX
        --subnet-handler-id "unused"
        --subnet-type "${subnet_type}"
        --replica-version-id "${version_commit}"
        --proposal-title "$PROPOSAL_TITLE"
        --summary-file "$PROPOSAL_SUMMARY_FILE"
        $node_ids)

    # print the command before executing it
    printf '%q ' "${cmd[@]}"
    "${cmd[@]}" --dry-run
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

    set -euo pipefail
    download_dir="$HOME/tmp/ic/$version_commit/guest-os/update-img"
    mkdir -p "$download_dir"
    UPDATE_URL="https://download.dfinity.systems/ic/${version_commit}/guest-os/update-img/update-img.tar.gz"
    download_img="${download_dir}/update-img.tar.gz"
    print_green "Downloading the update image from $UPDATE_URL to ${download_img}"

    CURL_HTTP_CODE=$(curl -C - --silent --write-out "%{http_code}" "$UPDATE_URL" --output "${download_img}")

    if [[ "$CURL_HTTP_CODE" != "416" && ("$CURL_HTTP_CODE" -lt 200 || "$CURL_HTTP_CODE" -gt 299) ]]; then
        error "curl failed with http_code $CURL_HTTP_CODE for $UPDATE_URL" >&2
    else
        print_green "curl $UPDATE_URL succeeded ($CURL_HTTP_CODE)"
    fi
    SHA256="$(sha256sum "$download_img" | cut -f 1 -d ' ')"
    print_green "SHA256-hashes of update-img.tar.gz: $SHA256"

    PROPOSAL_TITLE=${PROPOSAL_TITLE:-"Elect new replica binary revision (commit $version_commit)"}
    echo
    echo "Please provide the Release Notes (as bullets, without the title) for this release and ctrl-d when done"
    CHANGELOG="${CHANGELOG:-$(cat)}"
    PROPOSAL_SUMMARY_FILE=$(mktemp)
    cat >$PROPOSAL_SUMMARY_FILE <<_EOF
Elect new replica binary revision [$version_commit](https://github.com/dfinity/ic/tree/$version_commit)

# Release Notes:
$CHANGELOG
_EOF
    print_env_common_for_proposal
    print_param "replica version" "$version_commit"
    print_param "release package URL" "$UPDATE_URL"
    print_param "release package SHA256" "$SHA256"

    cmd=($IC_ADMIN $AUTH_PARAMS --nns-url="$NNS_URL"
        propose-to-update-elected-replica-versions
        --proposal-title "$PROPOSAL_TITLE"
        --summary-file $PROPOSAL_SUMMARY_FILE
        --replica-version-to-elect "$version_commit"
        --release-package-urls "$UPDATE_URL"
        --release-package-sha256-hex "$SHA256"
        --proposer $PROPOSER_NEURON_INDEX)

    # print the command before executing it
    printf '%q ' "${cmd[@]}"
    echo
    echo
    "${cmd[@]}" --dry-run
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
    version_commit_short=$(echo $version_commit | cut -c-8)

    set +euo pipefail

    check_version_commit_is_blessed "$version_commit"

    # get subnet id, to make sure the right subnet gets upgraded
    subnet_id=$(get_subnet_id_from_index "$subnet_index")
    subnet_id_short=$(echo $subnet_id | cut -d- -f1)

    set -euo pipefail
    PROPOSAL_TITLE=${PROPOSAL_TITLE:-"Update subnet $subnet_id_short to replica version $version_commit_short"}
    echo
    PROPOSAL_SUMMARY_FILE=$(mktemp)
    cat >$PROPOSAL_SUMMARY_FILE <<_EOF
Update subnet $subnet_id to replica version [$version_commit](https://dashboard.internetcomputer.org/release/$version_commit)
_EOF
    print_env_common_for_proposal

    cmd=($IC_ADMIN $ROLLOUT_AUTH_PARAMS --nns-url="$NNS_URL"
        propose-to-update-subnet-replica-version
        --proposal-title "$PROPOSAL_TITLE"
        --summary-file "$PROPOSAL_SUMMARY_FILE"
        $subnet_index $version_commit
        --proposer $ROLLOUT_NEURON_ID)

    # print the command before executing it
    printf '%q ' "${cmd[@]}"
    echo
    "${cmd[@]}" --dry-run
    echo

    do_you_want_to_continue

    echo "To interrupt, press Ctrl+C in the next 10 seconds."
    sleep 10
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
    "${cmd[@]}" --dry-run
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
    $IC_ADMIN --nns-url="$NNS_URL" $ic_admin_args get-topology | jq -e -r '.topology.subnets | to_entries[] | "\(.key)\t\(.value.records[] | .value.replica_version_id)\t\(.value.records[] | .value.subnet_type)"' | awk '{ print NR-1, $0 }'
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
