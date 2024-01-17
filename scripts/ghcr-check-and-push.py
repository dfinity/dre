import hashlib
import os
import subprocess

import git


def update_missing_images(targets):
    for target in targets:
        run_target(targets[target])


def run_target(target):
    subprocess.run(["bazel", "run", target["run_target"], "--", "--tag", target["sha256"]], check=True)

    subprocess.run(["bazel", "run", target["run_target"], "--", "--tag", target["commit"]], check=True)


def get_last_pushed_sha(targets, cmd):
    repo = git.Repo(".")
    prefix = "ghcr.io/dfinity/dre/"
    print("Will check following targets:", targets)

    to_update = {}

    for target in targets:
        full = prefix + target + ":" + targets[target]["sha256"]
        if check_image(full, cmd) == 0:
            print("Found tag", targets[target]["sha256"], "and will not push the new image")
            continue

        to_update[target] = {
            "sha256": targets[target]["sha256"],
            "commit": repo.head.commit.hexsha,
            "run_target": targets[target]["run_target"],
        }

    return to_update


def check_image(img, cmd):
    return subprocess.run([cmd, "pull", img]).returncode


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
target_sha_pairs = get_last_pushed_sha(targets, os.environ.get("CMD"))
update_missing_images(target_sha_pairs)
