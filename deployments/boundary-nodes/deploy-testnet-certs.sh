#!/usr/bin/env bash
set -euo pipefail

function err() {
    echo "[$(date +'%Y-%m-%dT%H:%M:%S%z')]: $*" >&2
}

function exit_usage() {
    err 'Usage: deploy-testnet-certs.sh --user <USERNAME> [--cert-name <CERTNAME>] [--test-cert] [--max-ssh <THREADS>] [--renew-days <DAYS>]'
    err '    --user <USERNAME>                     The username used to copy the certs to the testnet VM hosts.'
    err '    --test-cert                           Use the staging server to obtain (invalid) test certificates (default false)'
    err '    --max-ssh <THREADS>                   The maximum number of concurrent threads to use when copying files. (default 7)'
    err '    --renew-days <DAYS>                   The number days prior to expiry to renew an SSL cert. (default 30)'
    err '    --wildcard-superdomain                Add a wildcard of the superdomain to the certificate (e.g. *.testnet.dfinity.network). (default false)'
    err ''
    err 'Example:'
    err '    deployments/boundary-nodes/deploy-testnet-certs.sh --user danielbloom'
    err ''
    exit 1
}

function setup() {
    trap 'err "EXIT received, killing all jobs"; jobs -p | xargs -rn1 pkill -P >/dev/null 2>&1; exit 1' EXIT
    readonly REPO_ROOT="$(git rev-parse --show-toplevel)"

    while [ $# -gt 0 ]; do
        case "${1}" in
            --user)
                readonly USER="${2:-}"
                shift
                ;;
            --test-cert)
                readonly TEST_CERT=1
                ;;
            --max-ssh)
                MAX_CONCURRENT_SSH="${2:-}"
                if [[ -z "${MAX_CONCURRENT_SSH}" ]]; then exit_usage; fi
                shift
                ;;
            --renew-days)
                RENEW_DAYS="${2:-}"
                if [[ -z "${RENEW_DAYS}" ]]; then exit_usage; fi
                shift
                ;;
            --wildcard-superdomain)
                readonly WILDCARD_SUPERDOMAIN=1
                ;;
            -?*) exit_usage ;;
            *) exit_usage ;;
        esac
        shift
    done

    if [[ -z "${USER:-}" ]]; then
        err 'missing --user'
        exit_usage
    fi
    readonly MAX_CONCURRENT_SSH="${MAX_CONCURRENT_SSH:-7}"
    readonly RENEW_DAYS="${RENEW_DAYS:-30}"

    readonly TMPDIR=$(mktemp -d /tmp/deploy-testnet-certs.sh.XXXXXX)
    echo "Using temp dir '${TMPDIR}'"
}

function read_hosts() {
    local HOST_FILES
    readarray -d '' HOST_FILES < <(find "${REPO_ROOT}/ic/testnet/env/" -wholename "*/hosts" -print0)

    local PIDS=()
    echo "Getting info from ${#HOST_FILES[@]} 'hosts' files..."
    for host_file in "${HOST_FILES[@]}"; do
        {
            exec {hosts_fd}<>"${TMPDIR}/hosts"
            HOSTS="$("${host_file}" --list)"

            PHY_HOSTS=$(jq <<<"${HOSTS}" -r '(.physical_hosts.hosts // [])[]')
            DOMAIN=$(jq <<<"${HOSTS}" -r '.boundary.vars.system_domains // empty' | tr '[:upper:]' '[:lower:]')
            # This name used by Certbot for housekeeping and in file paths; it does not affect the content of the certificate itself.
            CERT_NAME=$(jq <<<"${HOSTS}" -r '.boundary.vars.cert_name // empty')

            if [[ -z "${CERT_NAME}" ]]; then
                err "'.boundary.vars.cert_name; was not defined in ${host_file}"
                exit 1
            fi

            mkdir -p "${TMPDIR}/${CERT_NAME}"
            flock ${hosts_fd}
            if [[ -n ${DOMAIN+x} ]]; then
                echo "${DOMAIN}" >>"${TMPDIR}/${CERT_NAME}/domains"
            fi
            if [[ -n ${PHY_HOSTS+x} ]]; then
                echo "${PHY_HOSTS}" >>"${TMPDIR}/${CERT_NAME}/hosts"
            fi
        } &
        PIDS+=("$!")
    done
    wait ${PIDS[@]}
}

function generate_certs() {
    # This name used by Certbot for housekeeping and in file paths; it does not affect the content of the certificate itself.
    local -r CERT_NAME="$1"
    local -r CERT_DIR="/etc/letsencrypt/live/${CERT_NAME}"
    local -r CERT_FILE="${CERT_DIR}/cert.pem"
    local -r CREDENTIALS='/etc/letsencrypt/.secret/cloudflare.ini'

    if ! sudo test -f "${CREDENTIALS}"; then
        err "Missing Credentials ('${CREDENTIALS}')"
        exit 1
    fi

    if sudo test -f "${CERT_FILE}"; then
        local END_DATE=$(sudo cat "${CERT_FILE}" | openssl x509 -noout -enddate | sed -e 's#notAfter=##')
        END_DATE=$(date -d "${END_DATE}" '+%s')
        local NOW=$(date '+%s')
        local DIFF="$(((${END_DATE} - ${NOW}) / (60 * 60 * 24)))"

        if [[ "${DIFF}" -le "0" ]]; then
            echo "Certificate expired, renewing."
        elif [[ "${DIFF}" -le "$((${RENEW_DAYS}))" ]]; then
            echo "Certificate will expire in $((${DIFF})) days, renewing."
        else
            echo "Certificate will expire in $((${DIFF})) days, not renewing."
            return
        fi
    else
        echo "No certificates found, generating one."
    fi

    echo "Reading domains..."
    local DOMAINS=()
    readarray -t DOMAINS <"${TMPDIR}/${CERT_NAME}/domains"
    echo "Read ${#DOMAINS[@]} domains."
    DOMAINS=($(tr ' ' '\n' <<<"${DOMAINS[@]/""/}" | sort -u | tr '\n' ' '))
    echo "Reduced to ${#DOMAINS[@]} domains."
    if [[ ${#DOMAINS[@]} -gt 33 ]]; then
        if [[ -z ${WILDCARD_SUPERDOMAIN+x} ]]; then
            err "Limited to 33 domains without '--wildcard-superdomain'."
            exit 1
        fi
        local SUPER_DOMAINS=()
        # Add the super domain, e.g. so we can get *.testnet.dfinity.network
        for DOMAIN in "${DOMAINS[@]}"; do
            SUPER_DOMAINS+=("$(sed -r 's/^[a-z0-9-]+\.//' <<<"${DOMAIN}")")
        done

        echo "Read ${#SUPER_DOMAINS[@]} super domains."
        SUPER_DOMAINS=($(tr ' ' '\n' <<<"${SUPER_DOMAINS[@]/""/}" | sort -u | tr '\n' ' '))
        echo "Reduced to ${#SUPER_DOMAINS[@]} super domains."
        if ((${#SUPER_DOMAINS[@]} + 2 * ${#DOMAINS[@]} > 100)); then
            err "Limited to 100 domains, found 2*${#DOMAINS[@]} domains and ${#SUPER_DOMAINS[@]} super domains."
            err ''
            err 'Domains:'
            err "${DOMAINS[@]}"
            err ''
            err 'Super Domains:'
            err "${SUPER_DOMAINS[@]}"
            exit 1
        elif [[ ${#DOMAINS[@]} -le 0 ]]; then
            err "No domains found."
            exit 1
        fi
        DOMAINS="$(printf "*.%s," "${SUPER_DOMAINS[@]}")$(printf "*.%s," "${DOMAINS[@]}")$(printf "*.raw.%s," "${DOMAINS[@]}" | sed 's/,$//g')"
    else
        DOMAINS="$(printf "%s," "${DOMAINS[@]}")$(printf "*.%s," "${DOMAINS[@]}")$(printf "*.raw.%s," "${DOMAINS[@]}" | sed 's/,$//g')"
    fi

    echo "Generating certs."
    sudo certbot certonly --cert-name "${CERT_NAME}" \
        --dns-cloudflare --dns-cloudflare-propagation-seconds 60 --dns-cloudflare-credentials "${CREDENTIALS}" \
        --domains "${DOMAINS}" ${TEST_CERT+"--test-cert" "--break-my-certs"}
}

function copy_to_hosts() {
    # This name used by Certbot for housekeeping and in file paths; it does not affect the content of the certificate itself.
    local -r CERT_NAME="$1"
    local -r CERT_DIR="/etc/letsencrypt/live/${CERT_NAME}"
    echo "Reading hosts..."
    local HOSTS=()
    readarray -t HOSTS <"${TMPDIR}/${CERT_NAME}/hosts"
    echo "Read ${#HOSTS[@]} hosts."
    HOSTS=($(tr ' ' '\n' <<<"${HOSTS[@]/""/}" | sort -u | tr '\n' ' '))
    echo "Reduced to ${#HOSTS[@]} hosts."

    echo "Copying certs."
    local -r CERTS=("fullchain.pem" "privkey.pem" "chain.pem")
    mkdir -p "${TMPDIR}/${CERT_DIR}"
    for CERT in "${CERTS[@]}"; do
        sudo cp "${CERT_DIR}/${CERT}" "${TMPDIR}/${CERT_DIR}/"
        sudo chmod go+r "${TMPDIR}/${CERT_DIR}/${CERT}"
    done

    # Copy all the certs onto the VM hosts
    local PIDS=()
    for HOST in "${HOSTS[@]}"; do
        if [[ "${#PIDS[@]}" -ge "${MAX_CONCURRENT_SSH}" ]]; then
            wait -n ${PIDS[@]}
            for f in "${PIDS[@]}"; do
                if ! kill -0 "$f" 2>/dev/null; then
                    PIDS=("${PIDS[@]/$f/}")
                fi
            done
        fi
        echo "Copying to ${HOST}"
        rsync --quiet --rsync-path='sudo rsync' --relative "${TMPDIR}/./${CERT_DIR}/" -rv "${USER}@${HOST}:/" || echo "${HOST} failed to copy '${CERT_DIR}'." &
        PIDS+=("$!")
    done
    wait ${PIDS[@]}
}

function execute() {
    read_hosts
    readarray -t CERT_NAMES < <(find "${TMPDIR}" -mindepth 1 -maxdepth 1 -type d -printf '%P\n')
    for CERT_NAME in "${CERT_NAMES[@]}"; do
        echo "Generating ${CERT_NAME}"
        generate_certs "${CERT_NAME}"
        copy_to_hosts "${CERT_NAME}"
    done
}

function cleanup() {
    trap - EXIT
    rm -rf "${TMPDIR}"
}

setup "$@"
execute
cleanup
