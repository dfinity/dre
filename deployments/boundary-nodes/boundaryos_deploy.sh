#!/usr/bin/env bash

# Re-deploy boundary node VMs using Ansible
#
# This script takes one positional argument:
#   <deployment_identifier>: The deployment referenced in `deployments/boundary-nodes/env/${DEPLOYMENT}`
#
# Dependencies:
# - >pip3 install ansible pyyaml gitpython peewee tenacity paramiko requests tabulate

# - Operating System: Ubuntu 20.04
# - >sudo apt install ansible coreutils jq mtools tar util-linux wget rclone
#
# - Operating System: MacOS 12.5
# - >brew install tree iproute2mac coreutil bash jq rclone dosfstools wget mtools gnu-tar
# - /usr/local/sbin/ must be in your path (for dosfstools)
#
# Also ensure updating the ic sub module
#  git submodule update --init --recursive

set -Euo pipefail

cd "$(dirname "$0")"
readonly REPO_ROOT="$(git rev-parse --show-toplevel)"
readonly BOUNDARY_NODES="${REPO_ROOT}/deployments/boundary-nodes"

function err() {
    echo "[$(date +'%Y-%m-%dT%H:%M:%S%z')]: $*" >&2
}

if [[ "${BASH_VERSINFO:-0}" -lt 4 ]]; then
    err "Bash 4+ is required"
    exit 1
fi

function exit_usage() {
    if (($# < 1)); then
        err 'Usage: boundaryos_deploy.sh [--git-revision <git_revision>] [--ansible-args <additional-args>] [--hosts-ini <hosts_override.ini>] <deployment_name> [--ssh-keys <directory_holding_SSH_key_files>] [--cert-dir <directory_holding_ssl_cert_files>]'
        err '    --git-revision <git_revision>    Deploy the testnet from the given git revision.'
        err '    --ansible-args <additional-args> Additional ansible args. Can be specified multiple times.'
        err '    --hosts-ini    <path>            Override the default ansible hosts.ini to set different testnet configuration'
        err '    --ssh-keys     <path>            Specify directory holding SSH authorized_key files'
        err "        (Default ${BOUNDARY_NODES}/ssh_keys_dfinity_nodes)"
        err '    --cert-dir        <path>         Specify directory holding ssl certificate files for nginx'
        err '    --nns-public-key <path>          Specify NNS public key pem file'
        err "        (Default ${BOUNDARY_NODES}/env/<DEPLOYMENT>/nns_public_key.pem)"
        err '    --prober-identity <path>         Specify an identity file for the prober'
        err '    --maxmind-license-key <key>      Specify maxmind license key for geolite2 database'
        err '    --cert-issuer-creds              Specify a credentials file for certificate-issuer'
        err '    --cert-issuer-identity           Specify an identity file for certificate-issuer'
        err '    --cert-issuer-enc-key            Specify an encryption key for certificate-issuer'
        err '    --cert-syncer-raw-domains        Specify a list of custom domains that bypass the service worker'
        err '    --pre-isolation-canisters        Specify a set of pre-domain-isolation canisters'
        err '    --ip-hash-salt                   Specify a salt for hashing IP values'
        err '    --logging-url                    Specify an endpoint for our logging backend'
        err '    --logging-user                   Specify a user for our logging backend'
        err '    --logging-password               Specify a password for our logging backend'
        err '    --logging-2xx-sample-rate        Specify a sampling rate for logging 2XX requests (1 / N)'
        err '        (Default 100, meaning 1 request out of 100 is logged)'
        err '    --cf-lb-api-token                Cloudflare API token to control load balancer pools'
        err '    --cf-lb-account-id               Cloudflare account ID which owns the load balancer pools'
        err '    --slack-webhook-url              Slack webhook URL for notifications'
        err '    --annotations-config-path        Path for MySQL configuration used for storing annotations'
        err ''
        err 'To get the latest branch revision that has a disk image pre-built, you can use ic/gitlab-ci/src/artifacts/newest_sha_with_disk_image.sh'
        err 'Example (deploy latest master production):'
        err "     boundaryos_deploy.sh dev --git-revision \$(${REPO_ROOT}/ic/gitlab-ci/src/artifacts/newest_sha_with_disk_image.sh master)"
        exit 1
    fi
}

function ansible() {
    ansible-playbook ${ANSIBLE_ARGS[@]} "$@"
}

function dateFromEpoch() {
    if [[ "$(uname)" == "Darwin" ]]; then
        date -j -f '%s' "$1"
    else
        date --date="@$1"
    fi
}

function disk_image_exists() {
    curl --output /dev/null --silent --head --fail \
        "https://download.dfinity.systems/ic/${GIT_REVISION}/boundary-os/disk-img/disk-img.tar.gz"
}

# Send message to Slack quoting it first to be used in a JSON
function slack() {
    MSG=$(echo -n $1 | jq -Rsa .)
    curl -X POST -H 'Content-type: application/json' --data "{\"text\":${MSG}}" ${SLACK_WEBHOOK_URL} >/dev/null
}

function cf_lb_control() {
    CF_API_TOKEN="${CF_LB_API_TOKEN}" CF_ACCOUNT_ID="${CF_LB_ACCOUNT_ID}" ${BOUNDARY_NODES}/cf-lb-origin-control.py $1 $2
}

## Annotations

function annotation_mark_started() {
    if [[ -z "${ANNOTATIONS_CONFIG_PATH}" ]]; then
        return
    fi

    if ! which mysql >/dev/null 2>&1; then
        err "Annotations credentials provided, but missing mysql"
        exit 1
    fi

    local environment=$1
    local hostnames=$2
    local start_time=$(date +'%Y-%m-%d %H:%M:%S')

    local id=$(
        mysql --defaults-file="${ANNOTATIONS_CONFIG_PATH}" annotations --skip-column-names <<EOF
            -- Insert row
            INSERT INTO boundary_node_deployments (status, start_time, environment, hostnames)
            VALUES (
                'started',        -- status
                '${start_time}',  -- start_time
                '${environment}', -- environment
                '${hostnames}'    -- hostnames
            );

            -- Get ID
            SELECT LAST_INSERT_ID();
EOF
    )

    echo "${id}"
}

function annotation_mark_ended() {
    if [[ -z "${ANNOTATIONS_CONFIG_PATH}" ]]; then
        return
    fi

    if ! which mysql >/dev/null 2>&1; then
        err "Annotations credentials provided, but missing mysql"
        exit 1
    fi

    local id=$1
    local status=$2
    end_time=$(date +'%Y-%m-%d %H:%M:%S')

    mysql --defaults-file="${ANNOTATIONS_CONFIG_PATH}" annotations <<EOF
        UPDATE boundary_node_deployments
        SET
            status   = '${status}',
            end_time = '${end_time}'
        WHERE id = ${id};
EOF
}

GIT_REVISION="${GIT_REVISION:-$(git submodule | cut -c2- | awk '{print $1}')}"
ANSIBLE_ARGS=()

while [ $# -gt 0 ]; do
    case "${1}" in
        --git-revision)
            readonly GIT_REVISION="${2:-}"
            shift
            ;;
        --ssh-keys)
            SSH_KEYS_DIR="${2:-}"
            if [[ -z "${SSH_KEYS_DIR}" ]]; then
                err "SSH key dir not set"
                exit_usage
            fi
            shift
            ;;
        --certdir)
            CERT_DIR="${2:-}"
            if [[ -z "${CERT_DIR}" ]]; then
                err "certdir not set"
                exit_usage
            fi
            shift
            ;;
        --nns-public-key=)
            NNS_PUBLIC_KEY="${2:-}"
            if [[ -z "${NNS_PUBLIC_KEY}" ]]; then
                err "nns public key key dir not set"
                exit_usage
            fi
            shift
            ;;
        --prober-identity)
            readonly PROBER_IDENTITY="${2:-}"
            if [[ -z "${PROBER_IDENTITY}" ]]; then
                err "prober identity file not set"
                exit_usage
            fi
            shift
            ;;
        --maxmind-license-key)
            readonly MAXMIND_LICENSE_KEY_FILE="${2:-}"
            shift
            ;;
        --cert-issuer-creds)
            readonly CERTIFICATE_ISSUER_CREDENTIALS="${2:-}"
            shift
            ;;
        --cert-issuer-identity)
            readonly CERTIFICATE_ISSUER_IDENTITY="${2:-}"
            shift
            ;;
        --cert-issuer-enc-key)
            readonly CERTIFICATE_ISSUER_ENCRYPTION_KEY="${2:-}"
            shift
            ;;
        --cert-syncer-raw-domains)
            readonly CERTIFICATE_SYNCER_RAW_DOMAINS="${2:-}"
            shift
            ;;
        --pre-isolation-canisters)
            readonly PRE_ISOLATION_CANISTERS="${2:-}"
            shift
            ;;
        --ip-hash-salt)
            readonly IP_HASH_SALT="${2:-}"
            shift
            ;;
        --logging-url)
            readonly LOGGING_URL="${2:-}"
            shift
            ;;
        --logging-user)
            readonly LOGGING_USER="${2:-}"
            shift
            ;;
        --logging-password)
            readonly LOGGING_PASSWORD="${2:-}"
            shift
            ;;
        --logging-2xx-sample-rate)
            readonly LOGGING_2XX_SAMPLE_RATE="${2:-100}" # Default: log 1 request in 100 (1 / N)
            shift
            ;;
        --ansible-args)
            if [[ -z "${2:-}" ]]; then
                err "Ansible args not set"
                exit_usage
            fi
            ANSIBLE_ARGS+=($2)
            shift
            ;;
        --replicas-ipv6)
            readonly REPLICA_IPV6_OVERRIDE="${2:-}"
            if [[ -z "${REPLICA_IPV6_OVERRIDE}" ]]; then
                err "REPLICA_IPV6_OVERRIDE not set"
                exit_usage
            fi
            shift
            ;;
        --hosts-ini)
            if [[ -z "${2:-}" ]]; then exit_usage; fi
            # This environment variable will be picked up by the Ansible inventory generation script.
            # No further action is required to use the custom HOSTS_INI file.
            export HOSTS_INI_FILENAME="${2}"
            shift
            ;;
        --cf-lb-api-token)
            readonly CF_LB_API_TOKEN="${2:-}"
            if [[ -z "${CF_LB_API_TOKEN}" ]]; then
                err "--cf-lb-api-token not set"
                exit_usage
            fi
            shift
            ;;
        --cf-lb-account-id)
            readonly CF_LB_ACCOUNT_ID="${2:-}"
            if [[ -z "${CF_LB_ACCOUNT_ID}" ]]; then
                err "--cf-lb-account-id not set"
                exit_usage
            fi
            shift
            ;;
        --slack-webhook-url)
            readonly SLACK_WEBHOOK_URL="${2:-}"
            if [[ -z "${SLACK_WEBHOOK_URL}" ]]; then
                err "--slack-webhook-url not set"
                exit_usage
            fi
            shift
            ;;
        --annotations-config-path)
            readonly ANNOTATIONS_CONFIG_PATH="${2:-}"
            shift
            ;;

        -?*) exit_usage ;;
        *) DEPLOYMENT="$1" ;;
    esac
    shift
done

if [[ -z "${GIT_REVISION:-}" ]]; then
    err "ERROR: GIT_REVISION not set."
    err "Please provide the GIT_REVISION as env. variable or the command line with --git-revision <value>"
    exit_usage
fi

if [[ -z "${DEPLOYMENT:-}" ]]; then
    err "ERROR: No deployment specified."
    exit_usage
fi

if [[ -z "${MAXMIND_LICENSE_KEY_FILE:-}" ]]; then
    err "ERROR: MaxMind license key specified."
    exit_usage
elif [[ ! -f "${MAXMIND_LICENSE_KEY_FILE}" ]]; then
    err "ERROR: MaxMind license key file ${MAXMIND_LICENSE_KEY_FILE} not found."
    exit 1
fi
readonly MAXMIND_LICENSE_KEY="$(cat "${MAXMIND_LICENSE_KEY_FILE}")"

if [[ -z "${SSH_KEYS_DIR:-}" ]]; then
    SSH_KEYS_DIR="${BOUNDARY_NODES}/ssh_keys_dfinity_nodes"
    err 'SSH_KEYS_DIR directory not specified.'
fi
echo "Using SSH_KEYS_DIR ${SSH_KEYS_DIR}"

if [[ ! -z "${CERT_DIR:=""}" ]]; then
    if [[ ! -f ${CERT_DIR}/fullchain.pem ]] || [[ ! -f ${CERT_DIR}/privkey.pem ]] || [[ ! -f ${CERT_DIR}/chain.pem ]]; then
        err "ERROR: ${CERT_DIR} fullchain.pem, privkey.pem, chain.pem files not found."
        err "https://eff-certbot.readthedocs.io/en/stable/using.html#where-are-my-certificates"
        exit_usage
    fi
    echo "Using CERT_DIR ${CERT_DIR}"
fi

# Check that the ansible file exists
HOSTS_DIR="${BOUNDARY_NODES}/env/${DEPLOYMENT}"
if [[ -n ${HOSTS_INI_FILENAME=""} ]]; then
    HOSTS_INI_FILE_PATH="${HOSTS_DIR}/${HOSTS_INI_FILENAME}"
    if [[ ! -f "${HOSTS_INI_FILE_PATH}" ]]; then
        err "The Ansible inventory file does not exist, aborting: ${HOSTS_INI_FILE_PATH}"
        exit 1
    fi
else
    if [[ ! -f "${HOSTS_DIR}/hosts.ini" && ! -f "${HOSTS_DIR}/hosts.yml" ]]; then
        err "The Ansible inventory file does not exist, aborting: ${HOSTS_DIR}/hosts.ini|yml"
        exit 1
    fi
fi

if [[ -z "${NNS_PUBLIC_KEY:-}" ]]; then
    NNS_PUBLIC_KEY="${HOSTS_DIR}/nns_public_key.pem"
    err "NNS_PUBLIC_KEY not specified."
fi
echo "Using NNS_PUBLIC_KEY ${NNS_PUBLIC_KEY}"

for i in {1..60}; do
    if disk_image_exists; then
        echo "Disk image found for ${GIT_REVISION}"
        break
    fi
    echo "Disk image not available for ${GIT_REVISION}, waiting 30s for it to be built by the CI ($i/60)"
    sleep 30
done
if [[ $i -ge 60 ]]; then
    echo "Disk image not found for ${GIT_REVISION}, giving up"
    exit 1
fi

echo "Deploying to ${DEPLOYMENT} from git revision ${GIT_REVISION}"

STARTTIME="$(date '+%s')"
echo "**** Deployment start time: $(dateFromEpoch "${STARTTIME}")"

echo "-------------------------------------------------------------------------------
**** Local IPv4 address information:

$(ip -4 address show | grep -vE 'valid_lft')

-------------------------------------------------------------------------------
**** Local IPv6 address information:

$(ip -6 address show | grep -vE 'valid_lft|fe80::')

-------------------------------------------------------------------------------"

BN_MEDIA_PATH="${REPO_ROOT}/ic/artifacts/boundary-guestos/${DEPLOYMENT}/${GIT_REVISION}"
INVENTORY="${BOUNDARY_NODES}/env/${DEPLOYMENT}/hosts"

ANSIBLE_ARGS+=(
    "-i" "${INVENTORY}"
    "-e" "bn_image_type="
    "-e" "ic_git_revision=${GIT_REVISION}"
    "-e" "bn_media_path=${BN_MEDIA_PATH}"
    "-e" "ic_boundary_node_image=boundary"
)

# Ensure we kill these on CTRL+C or failure
trap 'echo "EXIT received, killing all jobs"; jobs -p | xargs -rn1 pkill -P >/dev/null 2>&1; exit 1' EXIT

TMPDIR=$(mktemp -d /tmp/boundaryos-deploy.sh.XXXXXX)

echo "-------------------------------------------------------------------------------"
echo "**** Build USB sticks for boundary nodes"
cd "${REPO_ROOT}/ic/ic-os/boundary-guestos"
rm -rf "${BN_MEDIA_PATH}"
mkdir -p "${BN_MEDIA_PATH}"

{
    "${INVENTORY}" --media-json >"${BN_MEDIA_PATH}/${DEPLOYMENT}.json"

    # Generate the seed for the denylist
    readonly DENYLIST_URL=$(jq <"${BN_MEDIA_PATH}/${DEPLOYMENT}.json" -r '.bn_vars.denylist_url // empty')
    if [[ -n "${DENYLIST_URL}" ]]; then
        curl -s "${DENYLIST_URL}" \
            | jq -r '.canisters | to_entries | map("\"~^" + .key + (if .value.localities // [] | length > 0 then " (" + (.value.localities | join("|")) + ")$"  else " .*$" end) +  "\" \"1\";")[]' >"${TMPDIR}/denylist.map"
    fi
} &
JSON_AND_DENYLIST_PID=$!

# Generate NNS from factsdb registry
echo "**** Generate NNS_URLs"
{
    cd "${REPO_ROOT}"
    # Get NNS_URLs from the registry
    FACTS_ARGS=("--dump-fields" "ipv6" "--guests" "--deployment" "mainnet" "--dump-filter" "subnet=tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe")
    ./factsdb/main.py ${FACTS_ARGS[@]} \
        | sed "s/\(.*\)/http:\/\/[\1]:8080/" >${BN_MEDIA_PATH}/NNS_URLS

    NNS_NODE_COUNT=$(cat ${BN_MEDIA_PATH}/NNS_URLS | wc -l)

    if [ "$NNS_NODE_COUNT" -eq 0 ]; then
        echo "No NNS nodes were found in FactsDB"
        exit 1
    fi

    echo "NNS nodes found: $NNS_NODE_COUNT"

    # Uncomment this line to generate the NNS URL from the static factsdb.
    #./deployments/boundary-nodes/env/mercury/hosts --list | jq '.nns.hosts|.[]' | xargs -n1 -I{} grep {} ./factsdb/data/mercury_guests.csv | cut -d, -f 3 | sed "s/^/http:\/\/[/" | sed "s/$/]:8080/" >${BN_MEDIA_PATH}/NNS_URLS
} &
NNS_URLS_PID=$!

echo "-------------------------------------------------------------------------------"
echo "**** Fetching MaxMind GeoLite2 Databases"

readonly MAXMIND_DOWNLOAD_URL="https://download.maxmind.com/app/geoip_download"

# Country
readonly GEOLITE2_COUNTRY_DB="${TMPDIR}/GeoLite2-Country.mmdb"
{
    cd "${TMPDIR}"
    curl -sL "${MAXMIND_DOWNLOAD_URL}?edition_id=GeoLite2-Country&license_key=${MAXMIND_LICENSE_KEY}&suffix=tar.gz" \
        | tar -xzf -
    mv GeoLite2-Country_*/GeoLite2-Country.mmdb "${GEOLITE2_COUNTRY_DB}"
} &
MAX_MIND_COUNTRY_PID=$!

# City
readonly GEOLITE2_CITY_DB="${TMPDIR}/GeoLite2-City.mmdb"
{
    cd "${TMPDIR}"
    curl -sL "${MAXMIND_DOWNLOAD_URL}?edition_id=GeoLite2-City&license_key=${MAXMIND_LICENSE_KEY}&suffix=tar.gz" \
        | tar -xzf -
    mv GeoLite2-City_*/GeoLite2-City.mmdb "${GEOLITE2_CITY_DB}"
} &
MAX_MIND_CITY_PID=$!

echo "-------------------------------------------------------------------------------"
echo "**** Building Configuration Partition"

pushd ${REPO_ROOT}/ic
git fetch --depth=1 git@gitlab.com:dfinity-lab/public/ic.git "${GIT_REVISION}" || true
git checkout "${GIT_REVISION}"

ERRORS=()
wait ${JSON_AND_DENYLIST_PID} || ERRORS+=("Failed to get denylist")
wait ${NNS_URLS_PID} || ERRORS+=("Failed to get NNS_URLS")
wait ${MAX_MIND_COUNTRY_PID} || ERRORS+=("Failed to get MaxMind country db")
wait ${MAX_MIND_CITY_PID} || ERRORS+=("Failed to get MaxMind city db")
if [[ ${#ERRORS[@]} -ne 0 ]]; then
    for error in "${ERRORS[@]}"; do
        err "${error}"
    done
    exit 1
fi

BUILD_DEPLOYMENT_ARGS=(
    "--env=${DEPLOYMENT}"
    "--input=${BN_MEDIA_PATH}/${DEPLOYMENT}.json"
    "--output=${BN_MEDIA_PATH}"
    "--nns_url=${BN_MEDIA_PATH}/NNS_URLS"
    "--ssh=${SSH_KEYS_DIR}"
    "--geolite2-country-db=${GEOLITE2_COUNTRY_DB}"
    "--geolite2-city-db=${GEOLITE2_CITY_DB}"
    "--nns_public_key=${NNS_PUBLIC_KEY}"
    # Uncomment this line to deploy old git revisions
    #"--git-revision=${GIT_REVISION}"
)

if [[ -f "${TMPDIR}/denylist.map" ]]; then
    BUILD_DEPLOYMENT_ARGS+=("--denylist=${TMPDIR}/denylist.map")
fi

if [[ -n "${REPLICA_IPV6_OVERRIDE:-}" ]]; then
    BUILD_DEPLOYMENT_ARGS+=("--replicas-ipv6=${REPLICA_IPV6_OVERRIDE}")
fi

if [[ -n "${CERT_DIR}" ]]; then
    BUILD_DEPLOYMENT_ARGS+=("--certdir=${CERT_DIR}")
fi

if [[ -n "${PROBER_IDENTITY:-}" ]]; then
    BUILD_DEPLOYMENT_ARGS+=("--prober-identity=${PROBER_IDENTITY}")
fi

if [[ -n "${CERTIFICATE_ISSUER_CREDENTIALS:-}" ]]; then
    if [[ -z "${CERTIFICATE_ISSUER_IDENTITY:-}" || -z "${CERTIFICATE_ISSUER_ENCRYPTION_KEY}" ]]; then
        err "missing certificate issuer identity and/or encryption key"
        exit_usage
    fi

    BUILD_DEPLOYMENT_ARGS+=("--cert-issuer-creds=${CERTIFICATE_ISSUER_CREDENTIALS}")
    BUILD_DEPLOYMENT_ARGS+=("--cert-issuer-identity=${CERTIFICATE_ISSUER_IDENTITY}")
    BUILD_DEPLOYMENT_ARGS+=("--cert-issuer-enc-key=${CERTIFICATE_ISSUER_ENCRYPTION_KEY}")
fi

if [[ -n "${CERTIFICATE_SYNCER_RAW_DOMAINS:-}" ]]; then
    BUILD_DEPLOYMENT_ARGS+=("--cert-syncer-raw-domains-file=${CERTIFICATE_SYNCER_RAW_DOMAINS}")
fi

if [[ -n "${PRE_ISOLATION_CANISTERS:-}" ]]; then
    BUILD_DEPLOYMENT_ARGS+=("--pre-isolation-canisters=${PRE_ISOLATION_CANISTERS}")
fi

if [[ -n "${IP_HASH_SALT:-}" ]]; then
    BUILD_DEPLOYMENT_ARGS+=("--ip-hash-salt=${IP_HASH_SALT}")
fi

if [[ -n "${LOGGING_URL:-}" && -n "${LOGGING_USER:-}" && -n "${LOGGING_PASSWORD:-}" ]]; then
    BUILD_DEPLOYMENT_ARGS+=("--logging-url=${LOGGING_URL}")
    BUILD_DEPLOYMENT_ARGS+=("--logging-user=${LOGGING_USER}")
    BUILD_DEPLOYMENT_ARGS+=("--logging-password=${LOGGING_PASSWORD}")
    BUILD_DEPLOYMENT_ARGS+=("--logging-2xx-sample-rate=${LOGGING_2XX_SAMPLE_RATE}")
fi

./ic-os/boundary-guestos/scripts/build-deployment.sh ${BUILD_DEPLOYMENT_ARGS[@]}
if [[ $? != 0 ]]; then
    err "failed to build deployment payload, aborting."
    exit 1
fi

rm -rf "${TMPDIR}"
echo "${BN_MEDIA_PATH}"
tree "${BN_MEDIA_PATH}"
popd
echo "-------------------------------------------------------------------------------"

readonly BOUNDARY_HOST_GROUPS=($(jq <"${BN_MEDIA_PATH}/${DEPLOYMENT}.json" -r '.datacenters | map(.boundary_nodes[]) | group_by(.batch // .host)[] | map(.host) | join(",")'))
declare -Ar BOUNDARY_IPS="($(jq <"${BN_MEDIA_PATH}/${DEPLOYMENT}.json" -r '.datacenters | map(.boundary_nodes[])[] | "[" + .host + "]=\"" + (.ipv4_address | split("/")[0]) + "\"" '))"

declare -ar DOMAINS=($(jq <"${BN_MEDIA_PATH}/${DEPLOYMENT}.json" -r '[.bn_vars.system_domains[], .bn_vars.application_domains[]] | unique[]'))
if [[ "${#DOMAINS[@]}" -eq 0 ]]; then
    err "No domains present in '${BN_MEDIA_PATH}/${DEPLOYMENT}.json'"
    exit 1
fi

declare -ar API_DOMAINS=($(jq <"${BN_MEDIA_PATH}/${DEPLOYMENT}.json" -r '[.bn_vars.api_domains[]] | unique[]'))
if [[ "${#API_DOMAINS[@]}" -eq 0 ]]; then
    err "No api domains present in '${BN_MEDIA_PATH}/${DEPLOYMENT}.json'"
    exit 1
fi

echo "Serving domains ${DOMAINS[@]} and api-domains ${API_DOMAINS[@]}"
cd "${REPO_ROOT}/ic/testnet/ansible"

## Slack Notifications

declare SLACK_HOSTS=""
declare -i SLACK_HOST_COUNT=0

for BOUNDARY_HOST_GROUP in "${BOUNDARY_HOST_GROUPS[@]}"; do
    IFS=',' read -r -a BOUNDARY_HOSTS <<<"${BOUNDARY_HOST_GROUP}"
    for BOUNDARY_HOST in "${BOUNDARY_HOSTS[@]}"; do
        ORIGIN=(${BOUNDARY_HOST//\./ })
        SLACK_HOSTS+=" \`${ORIGIN}\`"
        SLACK_HOST_COUNT+=1
    done
done

slack "*${DEPLOYMENT^^}*: Boundary Nodes deployment started, hash: \`${GIT_REVISION}\`, nodes (${SLACK_HOST_COUNT}):${SLACK_HOSTS}"

## Deployment

function deploy_group() {
    local BOUNDARY_HOST_GROUP=$1

    echo '**** Shutting down Boundary Node VMs'
    ansible icos_network_redeploy.yml \
        -e ic_state="shutdown" \
        -e "ic_media_path=${BN_MEDIA_PATH}/" \
        --limit "${BOUNDARY_HOST_GROUP}" \
        || true

    echo '**** Destroying Boundary Node VMs'
    ansible icos_network_redeploy.yml \
        -e ic_state="destroy" \
        -e "ic_media_path=${BN_MEDIA_PATH}/" \
        --limit "${BOUNDARY_HOST_GROUP}" \
        || true

    echo '**** Creating new Boundary Node VMs'
    ansible icos_network_redeploy.yml \
        -e ic_state="create" \
        -e "ic_media_path=${BN_MEDIA_PATH}/" \
        --limit "${BOUNDARY_HOST_GROUP}"

    if [[ $? != 0 ]]; then
        err "Failed to create Boundary Node VMs"
        return 1
    fi

    echo '**** Starting Boundary Node VMs'
    ansible icos_network_redeploy.yml \
        -e ic_state="start" \
        --limit "${BOUNDARY_HOST_GROUP}"

    if [[ $? != 0 ]]; then
        err "Failed to start Boundary Node VMs"
        return 1
    fi
}

function check_group_health() {
    local BOUNDARY_HOSTS=("$@")

    echo -n "**** Waiting for ${#BOUNDARY_HOSTS[@]}x 'HTTP 200's: "
    for BOUNDARY_HOST in "${BOUNDARY_HOSTS[@]}"; do
        function check_domain() {
            local DOMAIN=$1
            local BOUNDARY_HOST=$2

            while [[ "$(curl -s -o /dev/null -w '%{http_code}' "https://${DOMAIN}/api/v2/status" --resolve "${DOMAIN}:443:${BOUNDARY_IPS[${BOUNDARY_HOST}]}")" -ne 200 ]]; do
                sleep 15
                echo -n '.'
            done
            echo -n '✅'
        }

        for DOMAIN in "${DOMAINS[@]}"; do
            check_domain "${DOMAIN}" "${BOUNDARY_HOST}"
        done

        for DOMAIN in "${API_DOMAINS[@]}"; do
            check_domain "${DOMAIN}" "${BOUNDARY_HOST}"
        done
    done
}

for BOUNDARY_HOST_GROUP in "${BOUNDARY_HOST_GROUPS[@]}"; do
    echo "Processing hosts: ${BOUNDARY_HOST_GROUP}"
    IFS=',' read -r -a BOUNDARY_HOSTS <<<"${BOUNDARY_HOST_GROUP}"

    # Mark deployment as started
    annotation_id=$(annotation_mark_started "${DEPLOYMENT}" "${BOUNDARY_HOST_GROUP}")

    # Disable origins in Cloudflare
    for BOUNDARY_HOST in "${BOUNDARY_HOSTS[@]}"; do
        # Extract hostname from FQDN
        ORIGIN=(${BOUNDARY_HOST//\./ })
        echo "**** Disabling origin '${ORIGIN}' in Cloudflare"
        cf_lb_control disable "${ORIGIN}"
    done

    echo "**** Waiting for 60sec for the DNS TTL to expire"
    sleep 60

    # Run deployment
    deploy_group "${BOUNDARY_HOST_GROUP}"
    if [[ $? != 0 ]]; then
        # Mark deployment as failed
        annotation_mark_ended "${annotation_id}" 'failed'

        err "Deployment failed, aborting."
        exit 1
    fi

    # Check BN Health
    check_group_health "${BOUNDARY_HOSTS[@]}"

    echo ''

    # Re-enable origins in Cloudflare
    for BOUNDARY_HOST in "${BOUNDARY_HOSTS[@]}"; do
        # Extract hostname from FQDN
        ORIGIN=(${BOUNDARY_HOST//\./ })
        echo "Enabling origin ${ORIGIN} in Cloudflare"
        cf_lb_control enable "${ORIGIN}"
    done

    # Mark deployment as complete
    annotation_mark_ended "${annotation_id}" 'complete'

    echo ''
done

ENDTIME="$(date '+%s')"
echo "**** Completed deployment at $(dateFromEpoch "${ENDTIME}") (start time was $(dateFromEpoch "${STARTTIME}"))"
duration=$((ENDTIME - STARTTIME))
echo "**** $((duration / 60)) minutes and $((duration % 60)) seconds elapsed."

slack "*${DEPLOYMENT^^}*: Boundary Nodes deployment finished in \`$((duration / 60)) minutes and $((duration % 60)) seconds\`"
trap - EXIT
