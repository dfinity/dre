#!/usr/bin/env python3
import gzip
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.request
from hashlib import sha256
from pathlib import Path

canisters_dir = Path(os.path.dirname(os.path.realpath(__file__)))

canisters = {
    "governance": {
        "version": "5c0af72426c7eca863201c4853cb18dab504a140",
        "checksum": "3f352d9af3cae26948132e6fe8990a8b0897bb6fb9624ebcce3778d24c7c0ab0",
    },
    "registry": {
        "version": "89129b8212791d7e05cab62ff08eece2888a86e0",
        "checksum": "2d9cdf89939992056f38f09b966315debc8fa0102a29d8faefc567144a1a9d74",
    },
    "cycles-minting": {
        "version": "2ef275ecee0b8e2e6eba06d13ff4585ba8957f2e",
        "checksum": "3485e824cc07f919a21f3042baca7fa7bb1b93046a2432eb3143effe551669d4",
    },
}

canister_principals = {
    k: v["staging"] for k, v in json.load(open(canisters_dir / "canister_ids.json", "r", encoding="utf-8")).items()
}

for canister, info in canisters.items():
    version = info["version"]
    principal = canister_principals[canister]
    print(
        "Deploying {canister} canister ({principal}) at version {version}".format(
            canister=canister, principal=principal, version=version
        )
    )
    with tempfile.TemporaryDirectory() as tmpdirname:
        wasmPathGz = Path(tmpdirname) / "canister.wasm.gz"
        wasmPath = Path(tmpdirname) / "canister.wasm"
        canisterURL = "https://download.dfinity.systems/ic/{version}/canisters/{canister}-canister.wasm.gz".format(
            canister=canister, version=version
        )
        urllib.request.urlretrieve(canisterURL, wasmPathGz)
        with gzip.open(wasmPathGz, "rb") as f_in:
            with open(wasmPath, "wb") as f_out:
                shutil.copyfileobj(f_in, f_out)
        checksum = sha256(open(wasmPath, "rb").read()).hexdigest()
        if checksum != info["checksum"]:
            print(
                """Invalid checksum for {canister} canister:
                want: {want}
                got:  {got}
            """.format(
                    canister=canister, got=checksum, want=info["checksum"]
                )
            )
            sys.exit(1)
        curr_canister_info = subprocess.check_output(["dfx", "canister", "--network=staging", "info", canister]).decode(
            "utf8"
        )
        curr_canister_hash = list(filter(lambda l: l.startswith("Module hash"), curr_canister_info.splitlines()))[0]
        curr_canister_hash = curr_canister_hash.split()[-1][2:]
        if curr_canister_hash == info["checksum"]:
            print("Canister already has the expected checksum %s, skipping\n" % curr_canister_hash)
            continue
        subprocess.run(
            [
                canisters_dir / "ic-admin",
                "--secret-key-pem={}".format(
                    Path.home() / ".config" / "dfx" / "identity" / "bootstrap-super-leader" / "identity.pem"
                ),
                "--nns-url=http://[2600:3004:1200:1200:5000:62ff:fedc:fe3c]:8080",
                "propose-to-change-nns-canister",
                "--mode=upgrade",
                "--proposer=49",
                "--canister-id={}".format(principal),
                "--wasm-module-path={}".format(wasmPath),
                "--wasm-module-sha256={}".format(checksum),
                "--summary",
                "Updating the NNS canisters on staging",
            ],
            check=True,
        )
        print("Waiting for the installation to finish")
        time.sleep(6)
