#!/usr/bin/env python3

from sys import exit, argv
import os
import CloudFlare

# Origin fields that are returned by GET which are not needed when doing PATCH
ORIGIN_FIELDS_REMOVE = {"healthy", "failure_reason"}

# Required env vars
ENV_VARS_REQUIRED = {"CF_API_TOKEN", "CF_ACCOUNT_ID"}

def usage():
     print(f"Required env vars: {ENV_VARS_REQUIRED}")
     print(f'Usage: {argv[0]} <origin> <enable|disable>')
     exit(1)

missing = ENV_VARS_REQUIRED - set(os.environ)
if missing:
    print(f"Env vars are missing: {missing}")
    exit(1)

if len(argv) != 3:
    usage()

if argv[1] == "enable":
    enabled = True
elif argv[1] == "disable":
    enabled = False
else:
    usage()

origin = argv[2]
account_id = os.getenv("CF_ACCOUNT_ID")

cf = CloudFlare.CloudFlare()

# Load all pools
try:
    pools = cf.accounts.load_balancers.pools.get(account_id)
except Exception as e:
    print(f"Unable to get pools: {e}")
    exit(1)

# Find the pools where the given origin is a member
# If it's part of several pools - then all of them would be acted upon
pools_to_update = []
for pool in pools:
    for orig in pool["origins"]:
        if orig["name"] == origin:
            pools_to_update.append(pool)
            break

if not pools_to_update:
    print(f"Origin '{origin}' wasn't found in any of pools")
    exit(1)

# Loop over all pools to update
for pool in pools_to_update:
    # Prepare the updated `origins` dict with `enabled` field set for the given origin
    pool_updated = {"origins": []}
    for orig in pool["origins"]:
        if orig["name"] == origin:
            orig["enabled"] = enabled

        pool_updated['origins'].append({k: v for k, v in orig.items() if not k in ORIGIN_FIELDS_REMOVE})

    try:
        cf.accounts.load_balancers.pools.patch(account_id, pool['id'], data=pool_updated)
    except Exception as e:
        print(f"Unable to persist changes: {e}")
        exit(1)
