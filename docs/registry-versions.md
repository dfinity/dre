# Registry Versions Dump (dre registry)

This document describes how to use the DRE CLI to inspect Internet Computer Protocol (ICP) registry versions as raw records, in JSON, suitable for precise diffs and troubleshooting.

## Commands

- Dump a single version (flat list of records):

```bash
dre registry --dump-version 50000 | jq
# aliases: --json-version
```

- Dump a range of versions using Python-style indexing (inclusive), where -1 is the last version:

```bash
dre registry --dump-version-range -5 -1 > last5.json
# aliases: --json-version-range
```

- Dump ALL versions (warning: large):

```bash
dre registry --dump-version-range > all.json
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

