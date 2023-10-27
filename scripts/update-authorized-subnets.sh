#!/usr/bin/env bash

set -eEuo pipefail
source $(dirname "$0")/lib.sh

AUTHORIZED_SUBNETS=()
#AUTHORIZED_SUBNETS+=(snjp4-xlbw4-mnbog-ddwy6-6ckfd-2w5a2-eipqo-7l436-pxqkh-l6fuv-vae)  # 1, verified
#AUTHORIZED_SUBNETS+=(qxesv-zoxpm-vc64m-zxguk-5sj74-35vrb-tbgwg-pcird-5gr26-62oxl-cae)  # 2, verified
#AUTHORIZED_SUBNETS+=(pae4o-o6dxf-xki7q-ezclx-znyd6-fnk6w-vkv5z-5lfwh-xym2i-otrrw-fqe)  # 3, verified, whitelisted, with external devs
#AUTHORIZED_SUBNETS+=(4zbus-z2bmt-ilreg-xakz4-6tyre-hsqj4-slb4g-zjwqo-snjcc-iqphi-3qe)  # 4, verified, reserved for bitcoin testing
#AUTHORIZED_SUBNETS+=(w4asl-4nmyj-qnr7c-6cqq4-tkwmt-o26di-iupkq-vx4kt-asbrx-jzuxh-4ae)  # 5, verified
#AUTHORIZED_SUBNETS+=(io67a-2jmkw-zup3h-snbwi-g6a5n-rm5dn-b6png-lvdpl-nqnto-yih6l-gqe)  # 6, verified
#AUTHORIZED_SUBNETS+=(5kdm2-62fc6-fwnja-hutkz-ycsnm-4z33i-woh43-4cenu-ev7mi-gii6t-4ae)  # 7, verified, whitelisted, with external devs
#AUTHORIZED_SUBNETS+=(shefu-t3kr5-t5q3w-mqmdq-jabyv-vyvtf-cyyey-3kmo4-toyln-emubw-4qe)  # 8, verified, distrikt.io https://c7fao-laaaa-aaaae-aaa4q-cai.ic0.app/
#AUTHORIZED_SUBNETS+=(ejbmu-grnam-gk6ol-6irwa-htwoj-7ihfl-goimw-hlnvh-abms4-47v2e-zqe)  # 9, verified, whitelisted, with external devs
#AUTHORIZED_SUBNETS+=(eq6en-6jqla-fbu5s-daskr-h6hx2-376n5-iqabl-qgrng-gfqmv-n3yjr-mqe)  # 10, OpenChat https://6hsbt-vqaaa-aaaaf-aaafq-cai.ic0.app/
#AUTHORIZED_SUBNETS+=(csyj4-zmann-ys6ge-3kzi6-onexi-obayx-2fvak-zersm-euci4-6pslt-lae)  # 11
AUTHORIZED_SUBNETS+=(lspz2-jx4pu-k3e7p-znm7j-q4yum-ork6e-6w4q6-pijwq-znehu-4jabe-kqe) # 12
AUTHORIZED_SUBNETS+=(lhg73-sax6z-2zank-6oer2-575lz-zgbxx-ptudx-5korm-fy7we-kh4hl-pqe) # 13
# AUTHORIZED_SUBNETS+=(gmq5v-hbozq-uui6y-o55wc-ihop3-562wb-3qspg-nnijg-npqp5-he3cj-3ae) # 14, fleek 1
# AUTHORIZED_SUBNETS+=(pjljw-kztyl-46ud4-ofrj6-nzkhm-3n4nt-wi3jt-ypmav-ijqkt-gjf66-uae) # 15, fleek 2
# AUTHORIZED_SUBNETS+=(brlsh-zidhj-3yy3e-6vqbz-7xnih-xeq2l-as5oc-g32c4-i5pdn-2wwof-oae) # 16, fleek 3
# AUTHORIZED_SUBNETS+=(mpubz-g52jc-grhjo-5oze5-qcj74-sex34-omprz-ivnsm-qvvhr-rfzpv-vae) # 17, fleek 4, DSocial https://dwqte-viaaa-aaaai-qaufq-cai.ic0.app/
# AUTHORIZED_SUBNETS+=(qdvhd-os4o2-zzrdw-xrcv4-gljou-eztdp-bj326-e6jgr-tkhuc-ql6v2-yqe) # 18, removed from authorized due to high number of canisters https://dfinity.slack.com/archives/C01DB8MQ5M1/p1659535103982329
# AUTHORIZED_SUBNETS+=(jtdsg-3h6gi-hs7o5-z2soi-43w3z-soyl3-ajnp3-ekni5-sw553-5kw67-nqe) # 19
AUTHORIZED_SUBNETS+=(k44fs-gm4pv-afozh-rs7zw-cg32n-u7xov-xqyx3-2pw5q-eucnu-cosd4-uqe) # 20
AUTHORIZED_SUBNETS+=(opn46-zyspe-hhmyp-4zu6u-7sbrh-dok77-m7dch-im62f-vyimr-a3n2c-4ae) # 21
AUTHORIZED_SUBNETS+=(6pbhf-qzpdk-kuqbr-pklfa-5ehhf-jfjps-zsj6q-57nrl-kzhpd-mu7hc-vae) # 22
AUTHORIZED_SUBNETS+=(e66qm-3cydn-nkf4i-ml4rb-4ro6o-srm5s-x5hwq-hnprz-3meqp-s7vks-5qe) # 23
AUTHORIZED_SUBNETS+=(4ecnw-byqwz-dtgss-ua2mh-pfvs7-c3lct-gtf4e-hnu75-j7eek-iifqm-sqe) # 24
# AUTHORIZED_SUBNETS+=(yinp6-35cfo-wgcd2-oc4ty-2kqpf-t4dul-rfk33-fsq3r-mfmua-m2ngh-jqe) # 25
# AUTHORIZED_SUBNETS+=(w4rem-dv5e3-widiz-wbpea-kbttk-mnzfm-tzrc7-svcj3-kbxyb-zamch-hqe) # 26 is the Peoples party subnet
AUTHORIZED_SUBNETS+=(cv73p-6v7zi-u67oy-7jc3h-qspsz-g5lrj-4fn7k-xrax3-thek2-sl46v-jae) # 27
AUTHORIZED_SUBNETS+=(o3ow2-2ipam-6fcjo-3j5vt-fzbge-2g7my-5fz2m-p4o2t-dwlc4-gt2q7-5ae) # 28
# AUTHORIZED_SUBNETS+=(fuqsr-in2lc-zbcjj-ydmcw-pzq7h-4xm2z-pto4i-dcyee-5z4rz-x63ji-nae) # 29, unused, was used for bitcoin testing, holds the test tECDSA key
AUTHORIZED_SUBNETS+=(3hhby-wmtmw-umt4t-7ieyg-bbiig-xiylg-sblrt-voxgt-bqckd-a75bf-rqe) # 30
AUTHORIZED_SUBNETS+=(nl6hn-ja4yw-wvmpy-3z2jx-ymc34-pisx3-3cp5z-3oj4a-qzzny-jbsv3-4qe) # 31
# AUTHORIZED_SUBNETS+=(x33ed-h457x-bsgyx-oqxqf-6pzwv-wkhzr-rm2j3-npodi-purzm-n66cg-gae) # 32, SNS
# AUTHORIZED_SUBNETS+=(uzr34-akd3s-xrdag-3ql62-ocgoh-ld2ao-tamcv-54e7j-krwgb-2gm4z-oqe) # 33, Internet Identity, holds the prod tECDSA key
# AUTHORIZED_SUBNETS+=(2fq7c-slacv-26cgz-vzbx2-2jrcs-5edph-i5s2j-tck77-c3rlz-iobzx-mqe) # 34, OpenChat subnet 2, holds the test tECDSA key
# AUTHORIZED_SUBNETS+=(pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae) # 35, Large app subnet, holds a backup of the prod tECDSA key

# Check for prerequisites.
check_ic_admin

KEY_ID=$(load_hsm_key_id) || exit $?
HSM_SLOT=$(load_hsm_slot) || exit $?
DFX_HSM_PIN=$(load_hsm_pin) || exit $?

cmd=(ic-admin --nns-url https://ic0.app
    --use-hsm --pin "$DFX_HSM_PIN" --key-id "$KEY_ID" --slot "$HSM_SLOT"
    propose-to-set-authorized-subnetworks --proposer 40
    --summary "Motivation: Update the list of authorized subnets to balance the canister load"
    --subnets "${AUTHORIZED_SUBNETS[@]}"
)

print_red() {
    echo -e "\033[0;31m$*\033[0m" 1>&2
}

print_green() {
    echo -e "\033[0;32m$*\033[0m"
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

printf '%q ' "${cmd[@]}"
echo
"${cmd[@]}" --dry-run

do_you_want_to_continue

"${cmd[@]}"
