ANSIBLE_USER  = dfnadmin
REPO_ROOT     = $(shell git rev-parse --show-toplevel)
BN_DIR        = $(REPO_ROOT)/deployments/boundary-nodes
DEPLOY_SCRIPT = $(BN_DIR)/boundaryos_deploy.sh
SSH_KEYS_DIR  = $(BN_DIR)/ssh_keys_dfinity_nodes

SECRETS_DIR                  = $(BN_DIR)/secrets
CERTS_DIR                    = $(SECRETS_DIR)
PROBER_IDENTITY              = $(SECRETS_DIR)/prober_identity.pem
MAXMIND_LICENSE_KEY          = $(SECRETS_DIR)/maxmind_license.key
LETSENCRYPT_CERTS_DIR        = /etc/letsencrypt/live
CLOUDFLARE_CREDENTIALS_CERT  = $(SECRETS_DIR)/cloudflare.ini
CLOUDFLARE_CREDENTIALS_LB    = $(SECRETS_DIR)/cloudflare_lb.txt

# Certificate Issuance
CERT_ISSUER_CREDENTIALS_DEV  = $(SECRETS_DIR)/certificate_issuer.dev.conf
CERT_ISSUER_CREDENTIALS_PROD = $(SECRETS_DIR)/certificate_issuer.prod.conf
CERT_ISSUER_IDENTITY_DEV     = $(SECRETS_DIR)/certificate_issuer_identity.dev.pem
CERT_ISSUER_IDENTITY_PROD    = $(SECRETS_DIR)/certificate_issuer_identity.prod.pem
CERT_ISSUER_ENC_KEY_DEV      = $(SECRETS_DIR)/certificate_issuer_enc_key.dev.pem
CERT_ISSUER_ENC_KEY_PROD     = $(SECRETS_DIR)/certificate_issuer_enc_key.prod.pem
CERT_SYNCER_RAW_DOMAINS_DEV  = $(SECRETS_DIR)/certificate_syncer_raw_domains.dev.txt
CERT_SYNCER_RAW_DOMAINS_PROD = $(SECRETS_DIR)/certificate_syncer_raw_domains.prod.txt

# Domain Isolation
PRE_ISOLATION_CANISTERS        = $(SECRETS_DIR)/pre_isolation_canisters.txt
PRE_ISOLATION_CANISTERS_SHA256 = 117d6826729c91391f1ea4ea11c7ee5182bfade9b5f89dc92a6e8c1d362ffeea

# IP Hashing Salts
IP_HASH_SALT_DEV  = $(shell cat $(SECRETS_DIR)/ip_hash_salt.dev.txt)
IP_HASH_SALT_PROD = $(shell cat $(SECRETS_DIR)/ip_hash_salt.prod.txt)

# Logging
LOGGING_URL_DEV       = $(shell cat $(SECRETS_DIR)/logging.dev.txt  | grep URL      | cut -d'=' -f2)
LOGGING_USER_DEV      = $(shell cat $(SECRETS_DIR)/logging.dev.txt  | grep USER     | cut -d'=' -f2)
LOGGING_PASSWORD_DEV  = $(shell cat $(SECRETS_DIR)/logging.dev.txt  | grep PASSWORD | cut -d'=' -f2)
LOGGING_URL_PROD      = $(shell cat $(SECRETS_DIR)/logging.prod.txt | grep URL      | cut -d'=' -f2)
LOGGING_USER_PROD     = $(shell cat $(SECRETS_DIR)/logging.prod.txt | grep USER     | cut -d'=' -f2)
LOGGING_PASSWORD_PROD = $(shell cat $(SECRETS_DIR)/logging.prod.txt | grep PASSWORD | cut -d'=' -f2)

# Cloudflare Load Balancer
CF_LB_API_TOKEN  = $(shell cat $(CLOUDFLARE_CREDENTIALS_LB) | grep CF_API_TOKEN  | cut -d'=' -f2)
CF_LB_ACCOUNT_ID = $(shell cat $(CLOUDFLARE_CREDENTIALS_LB) | grep CF_ACCOUNT_ID | cut -d'=' -f2)

# Slack
SLACK_WEBHOOK_URL = $(shell cat $(SECRETS_DIR)/slack.txt)

# Annotations
ANNOTATIONS_CONFIG_PATH = $(SECRETS_DIR)/annotations.cfg

DEPLOY_IMAGE:
ifeq ($(DEPLOY_IMAGE),)
	$(info Finding latest image)
	$(eval DEPLOY_IMAGE  := $(shell cd $(REPO_ROOT)/ic ; ./gitlab-ci/src/artifacts/newest_sha_with_disk_image.sh origin/master 2>/dev/null))
endif

show-latest: DEPLOY_IMAGE
	$(info Latest image is `$(DEPLOY_IMAGE)`)

verify-pre-isolation-canisters:
	@echo "Verifying pre-isolation canisters list"
	@[ "$(shell cat $(PRE_ISOLATION_CANISTERS) | shasum -a 256)" = "$(PRE_ISOLATION_CANISTERS_SHA256)  -" ] \
		|| ("Verification failed" && exit 1)

certs-dev:
	sudo certbot certonly --non-interactive \
		--cert-name bn-dev \
		--domains 'testic0.app,*.testic0.app,*.raw.testic0.app,icp0.io,*.icp0.io,*.raw.icp0.io,icp-api.io' \
		--dns-cloudflare \
		--dns-cloudflare-propagation-seconds 60 \
		--dns-cloudflare-credentials "$(CLOUDFLARE_CREDENTIALS_CERT)"

	mkdir -p $(CERTS_DIR)/dev
	sudo bash -c "cp -f $(LETSENCRYPT_CERTS_DIR)/bn-dev/{fullchain,privkey,chain}.pem $(CERTS_DIR)/dev/"
	sudo bash -c "chown -R $(USER):$(USER) $(CERTS_DIR)/dev/{fullchain,privkey,chain}.pem"

certs-prod:
	sudo certbot certonly --non-interactive \
		--cert-name bn-prod \
		--domains 'boundary.dfinity.network,*.mainnet.dfinity.network,mainnet.dfinity.network,rosetta.dfinity.network,ic0.app,*.ic0.app,*.raw.ic0.app,icp0.io,*.icp0.io,*.raw.icp0.io,icp-api.io' \
		--dns-cloudflare \
		--dns-cloudflare-propagation-seconds 60 \
		--dns-cloudflare-credentials "$(CLOUDFLARE_CREDENTIALS_CERT)"

	mkdir -p $(CERTS_DIR)/prod
	sudo bash -c "cp -f $(LETSENCRYPT_CERTS_DIR)/bn-prod/{fullchain,privkey,chain}.pem $(CERTS_DIR)/prod/"
	sudo bash -c "chown -R $(USER):$(USER) $(CERTS_DIR)/prod/{fullchain,privkey,chain}.pem"

vms-dev: verify-pre-isolation-canisters DEPLOY_IMAGE certs-dev
	$(info Installing testic0.app with `$(DEPLOY_IMAGE)`)
	bash $(DEPLOY_SCRIPT) dev \
		--git-revision                  $(DEPLOY_IMAGE)                \
		--certdir                       $(CERTS_DIR)/dev               \
		--ssh-keys                      $(SSH_KEYS_DIR)                \
		--prober-identity               $(PROBER_IDENTITY)             \
		--maxmind-license-key           $(MAXMIND_LICENSE_KEY)         \
		--replicas-ipv6                 ./env/replica_ipv6             \
		--cert-issuer-creds             $(CERT_ISSUER_CREDENTIALS_DEV) \
		--cert-issuer-identity          $(CERT_ISSUER_IDENTITY_DEV)    \
		--cert-issuer-enc-key           $(CERT_ISSUER_ENC_KEY_DEV)     \
		--cert-syncer-raw-domains       ${CERT_SYNCER_RAW_DOMAINS_DEV} \
		--pre-isolation-canisters       $(PRE_ISOLATION_CANISTERS)     \
		--ip-hash-salt                  $(IP_HASH_SALT_DEV)            \
		--logging-url                   $(LOGGING_URL_DEV)             \
		--logging-user                  $(LOGGING_USER_DEV)            \
		--logging-password              $(LOGGING_PASSWORD_DEV)        \
		--logging-2xx-sample-rate       1                              \
		--cf-lb-api-token               $(CF_LB_API_TOKEN)             \
		--cf-lb-account-id              $(CF_LB_ACCOUNT_ID)            \
		--slack-webhook-url             "$(SLACK_WEBHOOK_URL)"         \
		--annotations-config-path       $(ANNOTATIONS_CONFIG_PATH)

vms-prod: verify-pre-isolation-canisters certs-prod
ifeq ($(DEPLOY_IMAGE),)
	$(warning `DEPLOY_IMAGE` is not set.)
	$(warning Please make with `DEPLOY_IMAGE=<build_id> make vms-prod`)
	$(warning You can use `make show-latest` to find the latest image.)
	$(error Failed to install ic0.app)
endif
	$(info Installing ic0.app with `$(DEPLOY_IMAGE)`)
	bash $(DEPLOY_SCRIPT) prod \
		--git-revision                  $(DEPLOY_IMAGE)                 \
		--certdir                       $(CERTS_DIR)/prod               \
		--ssh-keys                      $(SSH_KEYS_DIR)                 \
		--prober-identity               $(PROBER_IDENTITY)              \
		--maxmind-license-key           $(MAXMIND_LICENSE_KEY)          \
		--replicas-ipv6                 ./env/replica_ipv6              \
		--cert-issuer-creds             $(CERT_ISSUER_CREDENTIALS_PROD) \
		--cert-issuer-identity          $(CERT_ISSUER_IDENTITY_PROD)    \
		--cert-issuer-enc-key           $(CERT_ISSUER_ENC_KEY_PROD)     \
		--cert-syncer-raw-domains       ${CERT_SYNCER_RAW_DOMAINS_PROD} \
		--pre-isolation-canisters       $(PRE_ISOLATION_CANISTERS)      \
		--ip-hash-salt                  $(IP_HASH_SALT_PROD)            \
		--logging-url                   $(LOGGING_URL_PROD)             \
		--logging-user                  $(LOGGING_USER_PROD)            \
		--logging-password              $(LOGGING_PASSWORD_PROD)        \
		--logging-2xx-sample-rate       1                               \
		--cf-lb-api-token               $(CF_LB_API_TOKEN)              \
		--cf-lb-account-id              $(CF_LB_ACCOUNT_ID)             \
		--slack-webhook-url             "$(SLACK_WEBHOOK_URL)"          \
		--annotations-config-path       $(ANNOTATIONS_CONFIG_PATH)
