## How to regenerate the .pb files

The `.pb` files are generated from their `.textproto` equivalents.

To convert a `.textproto` into `.pb`, you must have `protoc` installed. Then, from this directory, run:

```
RS=../../../ic/rs protoc -I $RS/nns/governance/proto/ -I $RS/nns/common/proto -I $RS/types/base_types/proto -I $RS/rosetta-api/ledger_canister/proto --encode ic_nns_governance.pb.v1.Governance $RS/nns/governance/proto/ic_nns_governance/pb/v1/governance.proto < initial-governance.textproto > initial-governance.pb
```

to turn `initial-governance.textproto` into `initial-governance.pb`
