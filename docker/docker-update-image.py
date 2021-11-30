#!/usr/bin/env python3
import filecmp
import fnmatch
import os
import pathlib
import pty
import re
import shlex
import shutil
import subprocess
import sys
import time

import git
from colorama import Fore
from colorama import init

git_repo = git.Repo(os.path.dirname(__file__), search_parent_directories=True)
repo_root = pathlib.Path(git_repo.git.rev_parse("--show-toplevel"))
IMAGE = "registry.gitlab.com/dfinity-lab/core/release-cli/ci-build"
os.chdir(repo_root)
# Init colorama
init(autoreset=True)


def ci_config_declared_image_digest():
    """Return the image digest that the CI config wants to use."""
    with open(repo_root / ".gitlab-ci.yml") as f:
        s = re.search(r"image:.+release-cli/ci-build@(.+)", f.read())
        if s:
            return s.group(1).strip()
    return ""


def local_image_sha256_unchecked():
    for i in range(100):
        digests = subprocess.check_output(["docker", "images", "--digests", "--format", "{{.Digest}}", IMAGE])
        digests = digests.decode("utf8").splitlines()
        if len(digests) > 0:
            if not digests[0] or digests[0] == "<none>":
                _print_magenta("Docker still didn't calculate the digest, waiting %s/100" % i)
                time.sleep(1)
                continue
            return digests[0]
        return ""


def local_image_sha256():
    r = subprocess.run(["docker", "inspect", "--format='{{index .RepoDigests 0}}", IMAGE], stdout=subprocess.PIPE)
    if r.returncode == 0:
        return r.stdout
    return ""


def find_ci_files():
    matches = []
    for root, dirs, filenames in os.walk(repo_root):
        dirs[:] = [d for d in dirs if d not in [".git", "ic", "deployments"]]
        for filename in fnmatch.filter(filenames, "*.yml"):
            matches.append(os.path.join(root, filename))
    return matches


def patch_ci_config_image_sha256(target_sha256):
    _print_green("Patching CI config to use image sha256 %s" % target_sha256)
    for f in find_ci_files():
        print("Patching CI config file %s" % f)
        subprocess.check_call(
            ["sed", "--in-place", "-e", f"s!/release-cli/ci-build.*$!/release-cli/ci-build@{target_sha256}!", f]
        )


def docker_build_image(cache_image):
    """Build the container image."""
    _print_green("Building the docker image...")
    os.environ["DOCKER_BUILDKIT"] = "1"
    cmd = [
        "docker",
        "build",
        "--progress=plain",
        "--cache-from",
        cache_image,
        "--tag",
        "release-cli:latest",
        "--tag",
        IMAGE,
        "-f",
        "docker/Dockerfile",
        ".",
    ]
    _print_green("$", shlex.join(cmd))
    exit_code = pty.spawn(cmd)
    if exit_code != 0:
        _print_red("Command failed with exit code %d" % exit_code)
        sys.exit(exit_code)


def docker_pull(ci_target_sha256):
    _print_magenta("docker pull '%s'" % IMAGE)
    exit_code = pty.spawn(["docker", "pull", f"{IMAGE}@{ci_target_sha256}"])
    if exit_code != 0:
        _print_red("Command failed with exit code %d" % exit_code)
        sys.exit(exit_code)


def docker_push():
    _print_magenta("docker push '%s'" % IMAGE)
    # Variable set automatically by GitLab
    registry_user = os.environ.get("CI_REGISTRY_USER")
    registry_pass = os.environ.get("CI_REGISTRY_PASSWORD")
    registry = os.environ.get("CI_REGISTRY")
    if registry_user and registry_pass and registry:
        print("Logging in to the docker registry...")
        out = subprocess.check_output(
            ["docker", "login", "--username", registry_user, "--password-stdin", registry],
            input=registry_pass.encode("utf8"),
        )
        print(out)
    else:
        _print_magenta("Cannot login to docker registry. Will try to push without logging in.")
        if not registry_user:
            _print_magenta("CI_REGISTRY_USER environment variable is not set.")
        if not registry_pass:
            _print_magenta("CI_REGISTRY_PASSWORD environment variable is not set.")
        if not registry:
            _print_magenta("CI_REGISTRY environment variable is not set.")

    exit_code = pty.spawn(["docker", "push", IMAGE])
    if exit_code != 0:
        _print_red("Command failed with exit code %d" % exit_code)
        sys.exit(exit_code)


def repo_changes_push():
    _print_magenta("git commit && git push")
    gitlab_token = os.environ.get("GITLAB_PUSH_TOKEN")
    if not gitlab_token:
        _print_magenta("GITLAB_PUSH_TOKEN environment variable is not set. Cannot push changes to the repo.")
        return
    git_status = git_repo.git.status("--short", ".gitlab-ci.yml", "docker")
    if not git_status:
        _print_green("No changes in the git repo")
        return
    print("Git repo is dirty")
    print(git_status)
    git_branch = os.environ.get("CI_COMMIT_REF_NAME")
    if not git_branch:
        _print_magenta("Cannot find git branch. Cannot push changes.")
        return
    print("Active git branch", git_branch)
    git_repo.git.checkout(git_branch)
    git_repo.config_writer().set_value("pull", "rebase", "true").release()
    git_repo.config_writer().set_value("rebase", "autoStash", "true").release()
    git_repo.config_writer().set_value("user", "name", "Release Team").release()
    git_repo.config_writer().set_value(
        "user", "email", "eng-release-bots-aaaafbmaump5gpag4pbjfuarry@dfinity.slack.com"
    ).release()
    # Update the remote URL to include the token with the write access
    origin = git_repo.remotes.origin
    remote_url = list(origin.urls)[0]
    remote_url = re.sub(r"https://(.+?)@", f"https://token:{gitlab_token}@", remote_url)
    origin.set_url(remote_url)
    print("Set the remote URL to: {}".format(remote_url))
    # Commit and push
    git_repo.git.add(all=True)
    git_repo.git.stash()
    origin.pull(git_branch, force=True)
    git_repo.git.stash("pop")
    git_repo.git.reset()
    git_repo.git.add(".gitlab-ci.yml", "docker", update=True)
    git_repo.git.commit("--no-verify", message="Automatically updated CI docker image")
    origin.push()
    _print_green("Pushed changes successfully")


def _are_dirs_identical(dir1, dir2):
    """Return True if two directories have identical tree content."""
    compared = filecmp.dircmp(dir1, dir2)
    if compared.left_only or compared.right_only or compared.diff_files or compared.funny_files:
        _print_red(
            "dir diff found %s %s %s"
            % (dir1, dir2, (compared.left_only or compared.right_only or compared.diff_files or compared.funny_files))
        )
        return False
    for subdir in compared.common_dirs:
        if not _are_dirs_identical(os.path.join(dir1, subdir), os.path.join(dir2, subdir)):
            return False
    return True


def _print_color(color, *kwargs):
    if isinstance(kwargs, list) or isinstance(kwargs, tuple):
        print(color + " ".join(kwargs))
    else:
        print(color + kwargs)


def _print_green(*kwargs):
    _print_color(Fore.GREEN, *kwargs)


def _print_magenta(*kwargs):
    _print_color(Fore.MAGENTA, *kwargs)


def _print_red(*kwargs):
    _print_color(Fore.RED, *kwargs)


def main():
    local_sha256 = local_image_sha256_unchecked()
    ci_target_sha256 = ci_config_declared_image_digest()
    if not ci_target_sha256.startswith("sha256:") or local_sha256 != ci_target_sha256:
        _print_magenta("local_sha256 '%s' != ci_target_sha256 '%s'" % (local_sha256, ci_target_sha256))
        docker_build_image(cache_image=f"{IMAGE}@{ci_target_sha256}")
        docker_push()
        local_sha256 = local_image_sha256_unchecked()
        patch_ci_config_image_sha256(local_sha256)
        repo_changes_push()
        sys.exit(0)

    _print_green("local_sha256 '%s' == ci_target_sha256 '%s'" % (local_sha256, ci_target_sha256))
    _print_green("Checking if the 'docker' subdir in the repo changed from the one in the image")

    docker_pull(ci_target_sha256)
    container_id = subprocess.check_output(["docker", "create", f"{IMAGE}@{ci_target_sha256}"]).decode("utf8").strip()
    DOCKER_SUBDIR_FROM_IMAGE = pathlib.Path("target/docker_contents_from_image")
    DOCKER_SUBDIR_FROM_IMAGE.mkdir(parents=True, exist_ok=True)
    shutil.rmtree(DOCKER_SUBDIR_FROM_IMAGE, ignore_errors=True)
    subprocess.check_call(["docker", "cp", f"{container_id}:docker", DOCKER_SUBDIR_FROM_IMAGE])
    subprocess.check_call(["docker", "rm", container_id])

    if _are_dirs_identical("docker", DOCKER_SUBDIR_FROM_IMAGE):
        _print_green("Subdir 'docker' unchanged in the image. Ending here.")
        sys.exit(0)

    _print_magenta("Subdir 'docker' has changes, updating the docker image.")

    # Something changed in the docker config, recreate the image and push it
    docker_build_image(cache_image=f"{IMAGE}@{ci_target_sha256}")
    docker_push()
    local_sha256 = local_image_sha256_unchecked()
    patch_ci_config_image_sha256(local_sha256)
    repo_changes_push()


if __name__ == "__main__":
    main()
