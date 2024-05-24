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

import git
from colorama import Fore
from colorama import init

git_repo = git.Repo(os.path.dirname(__file__), search_parent_directories=True)
repo_root = pathlib.Path(git_repo.git.rev_parse("--show-toplevel"))
IMAGE = "registry.gitlab.com/dfinity-lab/core/release/ci-build"
os.chdir(repo_root)
# Init colorama, strip=False forces colors even on non-interactive terminals, such as the CI logs
init(autoreset=True, strip=False)


def ci_config_declared_image_digest():
    """Return the image digest that the CI config wants to use."""
    with open(repo_root / ".gitlab-ci.yml", encoding="utf8") as f:
        s = re.search(r"image:.+release/ci-build@(.+)", f.read())
        if s:
            return s.group(1).strip()
    return ""


def local_image_sha256_unchecked(creator="docker"):
    """Return a tuple of the latest image digest and a set of all image digests."""
    digests = subprocess.check_output([creator, "images", "--digests", "--format", "{{.Digest}}", IMAGE])
    digests = digests.decode("utf8").splitlines()
    if len(digests) > 0:
        return (digests[0], set([d for d in digests if d.startswith("sha256:")]))
    return ("", set())


def find_ci_files():
    matches = []
    for root, dirs, filenames in os.walk(repo_root):
        dirs[:] = [d for d in dirs if d not in [".git"]]
        for filename in fnmatch.filter(filenames, "*.yml"):
            matches.append(os.path.join(root, filename))
    return matches


def patch_ci_config_image_sha256(target_sha256):
    if not isinstance(target_sha256, str) or not target_sha256.startswith("sha256:"):
        _print_red("Refusing to patch the CI config to use invalid image digest %s" % target_sha256)
        sys.exit(1)
    _print_green("Patching CI config to use image sha256 %s" % target_sha256)
    for f in find_ci_files():
        print("Patching CI config file %s" % f)
        subprocess.check_call(
            ["sed", "--in-place", "-e", f"s!/release/ci-build.*$!/release/ci-build@{target_sha256}!", f]
        )


def docker_build_image(cache_image, builder="docker"):
    """Build the container image."""
    _print_green("Building the docker image...")
    os.environ["DOCKER_BUILDKIT"] = "1"
    progress = ["--progress=plain"] if builder == "docker" else []
    cmd = (
        [
            builder,
            "build",
        ]
        + progress
        + [
            "--cache-from",
            cache_image,
            "--tag",
            "release:latest",
            "--tag",
            IMAGE,
            "-f",
            "docker/Dockerfile",
            ".",
        ]
    )
    _print_green("$", shlex.join(cmd))
    exit_code = pty.spawn(cmd)
    if exit_code != 0:
        _print_red("Command failed with exit code %d" % exit_code)
        sys.exit(exit_code)


def docker_pull(ci_target_sha256, creator="docker"):
    _print_magenta("docker pull '%s'" % IMAGE)
    exit_code = pty.spawn([creator, "pull", f"{IMAGE}@{ci_target_sha256}"])
    if exit_code != 0:
        _print_red("Command failed with exit code %d" % exit_code)
        sys.exit(exit_code)


def docker_push(creator="docker"):
    _print_magenta("docker push '%s'" % IMAGE)
    # Variable set automatically by GitLab
    registry_user = os.environ.get("CI_REGISTRY_USER")
    registry_pass = os.environ.get("CI_REGISTRY_PASSWORD")
    registry = os.environ.get("CI_REGISTRY")
    if registry_user and registry_pass and registry:
        print("Logging in to the docker registry...")
        out = subprocess.check_output(
            [creator, "login", "--username", registry_user, "--password-stdin", registry],
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

    exit_code = pty.spawn([creator, "push", IMAGE])
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
    git_repo.config_writer().set_value("pull", "rebase", "true").release()
    git_repo.config_writer().set_value("rebase", "autoStash", "true").release()
    git_repo.config_writer().set_value("user", "name", "Release Team").release()
    git_repo.config_writer().set_value(
        "user", "email", "eng-release-bots-aaaafbmaump5gpag4pbjfuarry@dfinity.slack.com"
    ).release()
    git_repo.git.stash()
    print("Active git branch", git_branch)
    git_repo.git.checkout(git_branch)
    git_repo.git.stash("pop")
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
            "dir diff found: %s %s %s"
            % (dir1, dir2, (compared.left_only or compared.right_only or compared.diff_files or compared.funny_files))
        )
        return False
    for subdir in compared.common_dirs:
        if not _are_dirs_identical(os.path.join(dir1, subdir), os.path.join(dir2, subdir)):
            return False
    return True


def _are_files_identical(file_list, local_cp_of_dir_image):
    """Return false if all files from the file_list are unmodified compared to the local_cp_of_dir_image."""
    for f in file_list:
        remote, local = f, local_cp_of_dir_image / f
        if not os.path.exists(remote) and not os.path.exists(local):
            # Files do not exist either in the container or in the local copy
            # so this is the same, and we continue merrily
            continue
        try:
            if not filecmp.cmp(remote, local):
                _print_red("file diff found: %s" % (f))
                return False
        except FileNotFoundError:
            _print_red("file diff found (missing in one side): %s" % (f))
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
    # Override these environment variables to buildah and podman
    # in order to establish the right tools to use here.
    builder = os.environ.get("BUILDER", "docker")
    creator = os.environ.get("CREATOR", "docker")

    os.environ["TERM"] = "xterm"  # to have colors in the child (pty spawned) processes
    local_sha256, local_sha256_set = local_image_sha256_unchecked(creator=creator)
    ci_target_sha256 = ci_config_declared_image_digest()
    if ci_target_sha256.startswith("sha256:"):
        docker_pull(ci_target_sha256, creator=creator)
    if not ci_target_sha256.startswith("sha256:") or ci_target_sha256 not in local_sha256_set:
        _print_magenta("ci_target_sha256 '%s' not in local_sha256 '%s'" % (ci_target_sha256, local_sha256_set))
        docker_build_image(cache_image=f"{IMAGE}@{ci_target_sha256}", builder=builder)
        docker_push()
        local_sha256, _ = local_image_sha256_unchecked(creator=creator)
        patch_ci_config_image_sha256(local_sha256)
        repo_changes_push()
        sys.exit(0)

    _print_green("ci_target_sha256 '%s' in local_sha256 '%s'" % (ci_target_sha256, local_sha256_set))
    _print_green("Checking if the 'docker' subdir in the repo changed from the one in the image")

    container_id = subprocess.check_output([creator, "create", f"{IMAGE}@{ci_target_sha256}"]).decode("utf8").strip()
    LOCAL_COPY_OF_IMAGE_SUBDIRS = pathlib.Path("target/check_docker_image_change")
    shutil.rmtree(LOCAL_COPY_OF_IMAGE_SUBDIRS, ignore_errors=True)
    LOCAL_COPY_OF_IMAGE_SUBDIR_DOCKER = LOCAL_COPY_OF_IMAGE_SUBDIRS / "docker"
    LOCAL_COPY_OF_IMAGE_SUBDIR_DOCKER.mkdir(parents=True, exist_ok=True)
    file_deps = ["Pipfile", "Pipfile.lock", "/opt/pyenv/pyproject.toml", "/opt/pyenv/poetry.lock"]
    for f in file_deps:
        p = subprocess.run([creator, "cp", f"{container_id}:{f}", LOCAL_COPY_OF_IMAGE_SUBDIRS], check=False)
        if p.returncode not in [0, 125, 1]:  # 125 = ENOENT in Buildah, 1 = ENOENT in Docker.
            p.check_returncode()
    subprocess.check_call([creator, "cp", f"{container_id}:docker", LOCAL_COPY_OF_IMAGE_SUBDIRS])
    subprocess.check_call([creator, "rm", container_id])

    if _are_dirs_identical("docker", LOCAL_COPY_OF_IMAGE_SUBDIR_DOCKER) and _are_files_identical(
        file_deps, LOCAL_COPY_OF_IMAGE_SUBDIRS
    ):
        _print_green("Docker image dependencies unchanged in the image. Ending here.")
        sys.exit(0)

    _print_magenta("Docker image dependencies changed, updating the docker image.")

    # Something changed in the docker config, recreate the image and push it
    docker_build_image(cache_image=f"{IMAGE}@{ci_target_sha256}", builder=builder)
    docker_push(creator=creator)
    local_sha256, _ = local_image_sha256_unchecked()
    patch_ci_config_image_sha256(local_sha256)
    repo_changes_push()


if __name__ == "__main__":
    main()
