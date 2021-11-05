#!/usr/bin/env bash

set -eEuo pipefail

AUTHORIZED_SUBNETS=()
AUTHORIZED_SUBNETS+=(lspz2-jx4pu-k3e7p-znm7j-q4yum-ork6e-6w4q6-pijwq-znehu-4jabe-kqe) # 12
AUTHORIZED_SUBNETS+=(lhg73-sax6z-2zank-6oer2-575lz-zgbxx-ptudx-5korm-fy7we-kh4hl-pqe) # 13
# AUTHORIZED_SUBNETS+=(gmq5v-hbozq-uui6y-o55wc-ihop3-562wb-3qspg-nnijg-npqp5-he3cj-3ae) # 14, fleek 1
# AUTHORIZED_SUBNETS+=(pjljw-kztyl-46ud4-ofrj6-nzkhm-3n4nt-wi3jt-ypmav-ijqkt-gjf66-uae) # 15, fleek 2
# AUTHORIZED_SUBNETS+=(brlsh-zidhj-3yy3e-6vqbz-7xnih-xeq2l-as5oc-g32c4-i5pdn-2wwof-oae) # 16, fleek 3
# AUTHORIZED_SUBNETS+=(mpubz-g52jc-grhjo-5oze5-qcj74-sex34-omprz-ivnsm-qvvhr-rfzpv-vae) # 17, fleek 4
AUTHORIZED_SUBNETS+=(qdvhd-os4o2-zzrdw-xrcv4-gljou-eztdp-bj326-e6jgr-tkhuc-ql6v2-yqe) # 18
# AUTHORIZED_SUBNETS+=(jtdsg-3h6gi-hs7o5-z2soi-43w3z-soyl3-ajnp3-ekni5-sw553-5kw67-nqe) # 19
AUTHORIZED_SUBNETS+=(k44fs-gm4pv-afozh-rs7zw-cg32n-u7xov-xqyx3-2pw5q-eucnu-cosd4-uqe) # 20
AUTHORIZED_SUBNETS+=(opn46-zyspe-hhmyp-4zu6u-7sbrh-dok77-m7dch-im62f-vyimr-a3n2c-4ae) # 21
AUTHORIZED_SUBNETS+=(6pbhf-qzpdk-kuqbr-pklfa-5ehhf-jfjps-zsj6q-57nrl-kzhpd-mu7hc-vae) # 22
AUTHORIZED_SUBNETS+=(e66qm-3cydn-nkf4i-ml4rb-4ro6o-srm5s-x5hwq-hnprz-3meqp-s7vks-5qe) # 23
AUTHORIZED_SUBNETS+=(4ecnw-byqwz-dtgss-ua2mh-pfvs7-c3lct-gtf4e-hnu75-j7eek-iifqm-sqe) # 24

echo "Inspect, remove the echo in this script and rerun"
echo ic-admin --nns-url https://nns.ic0.app --use-hsm --pin $(cat ~/.hsm-pin) --key-id 01 --slot 0 propose-to-set-authorized-subnetworks --proposer 40 --summary "Update the list of authorized subnets to balance the canister load" --subnets "${AUTHORIZED_SUBNETS[@]}"
