# Registry Versions Dump (dre registry)

This document describes how to use the DRE CLI to inspect Internet Computer Protocol (ICP) registry versions as raw records, in JSON, suitable for precise diffs and troubleshooting.

## Commands

- Dump a single version 50000 (flat list of records):

```bash
dre registry --dump-versions 50000 50001 | jq
```

- Dump a range of versions using Python-style indexing (end-exclusive), where -1 is the last index and omitted end means "to the end":

```bash
dre registry --dump-versions -5 > last5.json
# Indexing semantics (Python slicing, end-exclusive):
#   - positive indices are 1-based positions (registry is 1-based)
#   - 0 means start (same as omitting FROM)
#   - negative indices count from end (-1 is last)
#   - reversed ranges yield empty results
# Examples:
#   -5      -> last 5
#   -5 -1   -> last 4 (excludes the very last)
#   -1      -> last 1
#   0       -> all (from record 0 to the end)
```

- Dump ALL versions (warning: large):

```bash
dre registry --dump-versions > all.json
```

## Output Shape

Output is a flat JSON array of objects. Each object corresponds to a single registry record at a specific version:

```json
  {
    "version": 50000,
    "key": "node_record_cekdc-hmzri-3u3or-ei7ip-su7ck-3xt6e-zsse2-tgakq-rolmv-6crkh-hqe",
    "value": {
      "xnet": {
        "ip_addr": "2401:7500:ff1:20:6801:8fff:fe0c:cff4",
        "port": 2497
      },
      "http": {
        "ip_addr": "2401:7500:ff1:20:6801:8fff:fe0c:cff4",
        "port": 8080
      },
      "node_operator_id": {
        "bytes_base64": "K0aH3L0TlIkJORV0I4GwVU2GMLDZAv5U8Av1kAI=",
        "principal": "ri4lg-drli2-d5zpi-tsseq-soivo-qrydm-cvjwd-dbmgz-al7fj-4al6w-iae"
      },
      "chip_id": null,
      "hostos_version_id": "68fc31a141b25f842f078c600168d8211339f422",
      "public_ipv4_config": null,
      "domain": null,
      "node_reward_type": 5
    }
  },
[...]
```

