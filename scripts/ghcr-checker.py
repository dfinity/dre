import hashlib
import os
import subprocess

import git
import requests
from requests.auth import HTTPBasicAuth


def update_missing_images(targets):
    for target in targets:
        run_target(target)


def run_target(target):
    subprocess.run(["bazel", "run", target["run_target"], "--", "--tag", target["sha256"]], check=True)

    subprocess.run(["bazel", "run", target["run_target"], "--", "--tag", target["commit"]], check=True)


def get_last_pushed_sha(targets, access_token):
    repo = git.Repo(".")
    prefix = "https://ghcr.io/v2/dfinity/dre/"
    print("Will check following targets:", targets)

    response = requests.get(
        'https://ghcr.io/token?scope="repository:dfinity/dre:pull"', auth=HTTPBasicAuth("nikola-milosa", access_token)
    )
    if response.status_code != 200:
        print(response.text)
        exit(1)

    headers = {
        "Authorization": f"Bearer {response.json()['token']}",
        "Accept": ", ".join(
            ["application/vnd.docker.distribution.manifest.v2+json", "application/vnd.oci.image.manifest.v1+json"]
        ),
    }
    to_update = {}

    for target in targets:
        url = prefix + target + "/manifests/" + targets[target]["sha256"]
        response = requests.get(url, headers=headers)
        if response.status_code == 200:
            print("Found tag", target["sha256"], "and will not push the new image")
            continue

        to_update[target] = {
            "sha256": targets[target]["sha256"],
            "commit": repo.head.commit.hexsha,
            "run_target": targets[target]["run_target"],
        }

    return to_update


def get_bazel_targets():
    response = (
        subprocess.run(
            ["bazel", "query", "--noshow_progress", 'kind("oci_push", ...)'],
            text=True,
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        .stdout.strip()
        .split("\n")
    )

    print("Found the following targets:", response)

    targets = {}
    chunk_size = 8192

    for target in response:
        key = target.split("/")[-1].split(":")[0]
        sha256_hash = hashlib.sha256()
        with open("bazel-out/k8-opt/bin/" + target.split("//")[1].split(":")[0] + "/" + key, "rb") as f:
            for chunk in iter(lambda: f.read(chunk_size), b""):
                sha256_hash.update(chunk)
        targets[key] = {"sha256": sha256_hash.hexdigest(), "run_target": target}

    return targets


targets = get_bazel_targets()
target_sha_pairs = get_last_pushed_sha(targets, os.environ.get("GITHUB_TOKEN"))
print(target_sha_pairs)
