#!/usr/bin/env python3
import fnmatch
import os
import pathlib
import re
import subprocess
import sys

import git

git_repo = git.Repo(os.path.dirname(__file__), search_parent_directories=True)
repo_root = pathlib.Path(git_repo.git.rev_parse("--show-toplevel"))
GIT_EMAIL = "eng-release-bots-aaaafbmaump5gpag4pbjfuarry@dfinity.slack.com"


def find_ci_files():
    matches = []
    for root, dirs, filenames in os.walk(repo_root):
        dirs[:] = [d for d in dirs if d in ["gitlab-ci"]]
        for filename in fnmatch.filter(filenames, "*.yml"):
            matches.append(os.path.join(root, filename))
    return matches


def patch_ci_config_image_sha256(target_sha256):
    if not isinstance(target_sha256, str) or not target_sha256.startswith("sha256:"):
        print(f"Refusing to patch the CI config to use invalid image digest {target_sha256}")
        sys.exit(1)
    print(f"Patching CI config to use image {target_sha256}")
    for f in find_ci_files():
        print(f"Patching CI config file {f}")
        subprocess.check_call(
            [
                "sed",
                "--in-place",
                "-e",
                f"s!/release/ci-build/no-docker@sha256:.*$!/release/ci-build/no-docker@{target_sha256}!",
                f,
            ]
        )


def push_changes_to_repository(dry_run=True):
    if dry_run:
        print("Dry-run pushed to repository")
        return
    print("git commit && git push")
    gitlab_token = os.environ.get("GITLAB_PUSH_TOKEN")
    if not gitlab_token:
        print("GITLAB_PUSH_TOKEN environment variable is not set. Cannot push changes to the repo.")
        return
    git_status = git_repo.git.status("--short", *find_ci_files())
    if not git_status:
        print("No changes in the git repo")
        return
    print("Git repo is dirty")
    print(git_status)
    git_branch = os.environ.get("CI_COMMIT_REF_NAME")
    if not git_branch:
        print("Cannot find git branch. Cannot push changes.")
        return
    git_repo.config_writer().set_value("pull", "rebase", "true").release()
    git_repo.config_writer().set_value("rebase", "autoStash", "true").release()
    git_repo.config_writer().set_value("user", "name", "Release Team").release()
    git_repo.config_writer().set_value("user", "email", GIT_EMAIL).release()
    git_repo.git.stash()
    print("Active git branch", git_branch)
    git_repo.git.checkout(git_branch)
    git_repo.git.stash("pop")
    # Update the remote URL to include the token with the write access
    origin = git_repo.remotes.origin
    remote_url = list(origin.urls)[0]
    remote_url = re.sub(r"https://(.+?)@", f"https://token:{gitlab_token}@", remote_url)
    origin.set_url(remote_url)
    # Commit and push
    git_repo.git.add(all=True)
    git_repo.git.stash()
    origin.pull(git_branch, force=True)
    git_repo.git.stash("pop")
    git_repo.git.reset()
    git_repo.git.add(*find_ci_files(), update=True)
    git_repo.git.commit("--no-verify", message="Automatically updated CI image")
    origin.push()
    print("Pushed changes successfully")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Provide the sha to update to")
        sys.exit(1)
    target_sha = sys.argv[1]
    dry_run = not bool(os.getenv("CI"))

    patch_ci_config_image_sha256(target_sha)
    push_changes_to_repository(dry_run)
