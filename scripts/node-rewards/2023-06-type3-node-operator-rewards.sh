#!/bin/bash

# Zondax
# curl -sf https://dashboard.internal.dfinity.network/api/proxy/registry/mainnet/nodes | jq -r 'to_entries[] | .value | select (.operator.principal == "ap2qh-n5jrf-vpjj6-i5pap-qxnhr-4tnar-yodpw-kh6fy-c5w4z-gibij-5ae") | .principal'

release_cli propose update-node-operator-config \
    --node-operator-id ap2qh-n5jrf-vpjj6-i5pap-qxnhr-4tnar-yodpw-kh6fy-c5w4z-gibij-5ae \
    --rewardable-nodes '{"type3": 2}' \
    --summary "Set rewards for the following nodes:

* ufaoq-5b53e-ymjq6-erbjq-wmjhe-5auag-are62-3eulh-54p2q-gbayn-6ae
* lj6xb-t5tcb-dol3s-ay52b-dulse-zyezk-25qyv-l7yqj-4zrc2-hp5qz-sae    
"
# ==> https://dashboard.internetcomputer.org/proposal/122884
# ❯ release_cli get node-operator ap2qh-n5jrf-vpjj6-i5pap-qxnhr-4tnar-yodpw-kh6fy-c5w4z-gibij-5ae --json
#  INFO  release_cli::ic_admin > Using ic-admin: /home/sat/bin/ic-admin.revisions/c51cbbb8ab31bf4302f9f878badc42f28e4f6ed8/ic-admin
#  INFO  release_cli::ic_admin > running ic-admin:
# $ ic-admin \
#     --nns-url https://ic0.app/ \
#   get-node-operator \
#   ap2qh-n5jrf-vpjj6-i5pap-qxnhr-4tnar-yodpw-kh6fy-c5w4z-gibij-5ae \
#     --json
# {
#   "key": "node_operator_record_ap2qh-n5jrf-vpjj6-i5pap-qxnhr-4tnar-yodpw-kh6fy-c5w4z-gibij-5ae",
#   "version": 36364,
#   "value": {
#     "node_operator_principal_id": "ap2qh-n5jrf-vpjj6-i5pap-qxnhr-4tnar-yodpw-kh6fy-c5w4z-gibij-5ae",
#     "node_allowance": 0,
#     "node_provider_principal_id": "hzqcb-iiagd-4erjo-qn7rq-syqro-zztl6-cpble-atnkd-2c6bg-bxjoa-qae",
#     "dc_id": "zh5",
#     "rewardable_nodes": {
#       "type3": 2
#     },
#     "ipv6": null
#   }
# }

# Rivram
# curl -sf https://dashboard.internal.dfinity.network/api/proxy/registry/mainnet/nodes | jq -r 'to_entries[] | .value | select (.operator.principal == "zhlzs-2otly-4u7vw-qkz7m-2m7aw-bhwmv-yylta-7w4zi-yrrpj-g7eoa-2qe") | .principal'
release_cli propose update-node-operator-config \
    --node-operator-id eu5wc-g7r7l-zzy2o-227af-hfvin-orflw-4vdlf-j7gks-n5wrj-zezt7-tqe \
    --rewardable-nodes '{"type3": 1}' \
    --summary "Set rewards for the following nodes:

* yim23-u5r3y-yffo3-3hrab-vtvok-nw2qw-uzzbf-mvepc-fvnso-hjar4-6ae
"
# ==> https://dashboard.internetcomputer.org/proposal/122885
# Correction for Rivram
release_cli propose update-node-operator-config \
    --node-operator-id zhlzs-2otly-4u7vw-qkz7m-2m7aw-bhwmv-yylta-7w4zi-yrrpj-g7eoa-2qe \
    --rewardable-nodes '{"type3": 1}' \
    --summary "Correcting a mistake made in proposal [122885](https://dashboard.internetcomputer.org/proposal/122885), in which a wrong node operator principal was provided for following node:

* yim23-u5r3y-yffo3-3hrab-vtvok-nw2qw-uzzbf-mvepc-fvnso-hjar4-6ae
"
# ==> https://dashboard.internetcomputer.org/proposal/122982

# Anonstake
# curl -sf https://dashboard.internal.dfinity.network/api/proxy/registry/mainnet/nodes | jq -r 'to_entries[] | .value | select (.operator.principal == "eu5wc-g7r7l-zzy2o-227af-hfvin-orflw-4vdlf-j7gks-n5wrj-zezt7-tqe") | .principal'
release_cli propose update-node-operator-config \
    --node-operator-id eu5wc-g7r7l-zzy2o-227af-hfvin-orflw-4vdlf-j7gks-n5wrj-zezt7-tqe \
    --rewardable-nodes '{"type3": 12}' \
    --summary "Set rewards for the following nodes:

* 5zqhj-66qn7-he3p5-76gnj-hlsvl-ajlt5-k356i-fgm4m-5it4c-6m5ag-7qe
* c37f7-3shrz-2mk6g-e2zvp-3z6xc-uuk6r-6yegs-53uzt-c3vtf-dacsv-tae
* dnorv-obqlt-6fl2s-ylxsf-o6pwv-fssdj-63kdj-jjw5d-luhdg-xe2zn-2ae
* fr3jr-74qz3-jzmga-mpt7c-eipd7-numir-hpczg-egnrw-lrrrh-2f7ws-qqe
* g3ug2-wz6kb-unyk6-6mj4f-halk3-kkfvi-k33iz-fftob-qipom-t4kzr-qqe
* gj6gc-mbslq-bt63y-72b7h-xs2xw-6vyuj-frvp3-tx375-dsqx5-wem6o-vqe
* nijhr-z5may-w7sdi-v4roj-kclae-3pxhm-nqtqp-ofq6w-yhstt-4wirl-bae
* ocony-vhzun-3ygcw-3mck2-2knqd-d47bv-hi7jq-kbfc5-xf6ui-ou5c4-bqe
* qkcyr-pk3wb-n4w54-hofnn-mf6g6-uvhhj-lwzir-rw737-3ji4w-hqynu-vae
* u6e7b-mgtes-r5nay-mmrcn-p3rns-hj2fn-74dn7-cp2mu-kk24k-ljfnm-2ae
* vs5lv-6h44x-qfalu-aesc6-uehxl-zlwiu-5z7ik-c3kjy-6vtxl-3yz7x-qae
* y6xdi-6nbil-4w6ju-4vqux-wdl5m-uofuq-2hbv4-dels3-knu42-xievh-hae
"
# ==> https://dashboard.internetcomputer.org/proposal/122886
