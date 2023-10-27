This folder should be setup to contain all secrets locally.

Specifically the following files should be manually added to this folder (but **_<font color="red">⚠️ NEVER ⚠️</font>_** committed):

NOTE: Most of these files require two versions: `<name>.<env>.<ext>`, e.g `logging.dev.txt` and `logging.prod.txt`. See the Makefile for the specific usage examples.

- `prober_identity.pem`
- `maxmind_license.key`
- `certs/dev/*`
  - `chain.pem`
  - `fullchain.pem`
  - `privkey.pem`
- `certs/prod/*`
  - `chain.pem`
  - `fullchain.pem`
  - `privkey.pem`
- `pre_isolation_canisters.txt`
- `ip_hash_salt.txt`
- `logging.txt`
- `cloudflare.ini`
- `cloudflare_lb.txt`
- `slack.txt`
- `annotations.cfg`

Please refer to [the runbook](https://www.notion.so/Populate-Secrets-5f21b21395d340658f0d5c1969044db0) for instructions on how to populate these secrets.
