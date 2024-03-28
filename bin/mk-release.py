#!/usr/bin/env python3
import argparse
import difflib
import logging
import os
import pathlib
import re
import subprocess

# Make sure the script is run from the root of the repo
repo_root = pathlib.Path(__file__).resolve().parent.parent
if not (repo_root / "Cargo.toml").exists():
    raise SystemExit("This script must be run from the root of the repository")
os.chdir(repo_root)

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
log = logging.getLogger("mk-release")


def get_current_version():
    with open("VERSION", "r", encoding="utf8") as f:
        return f.read().strip()


def parse_args():
    parser = argparse.ArgumentParser(description="Update the version in the repo")
    parser.add_argument("new_version", type=str, help="New version")
    return parser.parse_args()


def patch_file(file_path, search_regex, replacement_string):
    log.info("Patching file %s", file_path)
    with open(file_path, "r", encoding="utf8") as f:
        contents = f.read()
    contents_new = re.sub(search_regex, replacement_string, contents, flags=re.MULTILINE)
    # Show difference between old and new contents
    for line in difflib.unified_diff(contents.splitlines(), contents_new.splitlines(), lineterm=""):
        log.info("  %s", line)
    with open(file_path, "w", encoding="utf8") as f:
        f.write(contents_new)
    subprocess.check_call(["git", "add", file_path])


def add_git_tag(tag_name):
    log.info("Creating git tag: %s", tag_name)
    subprocess.check_call(["git", "tag", tag_name])


def update_change_log(new_version):
    # call poetry run pychangelog generate to update CHANGELOG.md
    subprocess.check_call(["poetry", "run", "pychangelog", "generate"])
    # Add the CHANGELOG.md to the commit
    subprocess.check_call(["git", "add", "CHANGELOG.md"])
    # Commit the changes
    subprocess.check_call(["git", "commit", "-m", f"Release {new_version}"])
    # Push the new branch
    subprocess.check_call(["git", "push", "origin", "--force", f"release-{new_version}"])


def main():
    args = parse_args()
    current_version = get_current_version()
    # Check that the new version has format x.y.z
    if not re.match(r"\d+\.\d+\.\d+", args.new_version):
        raise SystemExit(f"New version needs to be provided in format x.y.z {args.new_version}")
    # Check that the new version is greater than the current version
    if args.new_version <= current_version:
        raise SystemExit(
            f"New version {args.new_version} needs to be greater than the current version {current_version}"
        )
    log.info("Updating version from %s to %s", current_version, args.new_version)
    subprocess.check_call(["git", "checkout", "main"])
    subprocess.check_call(["git", "pull"])
    # Create a new branch for the release
    subprocess.check_call(["git", "checkout", "-b", f"release-{args.new_version}"])
    patch_file("Cargo.toml", r'^version = "[\d\.]+"', f'version = "{args.new_version}"')
    patch_file("VERSION", f"^{current_version}$", args.new_version)
    new_git_tag = f"v{args.new_version}"
    add_git_tag(new_git_tag)
    update_change_log(args.new_version)
    subprocess.check_call(["git", "push", "origin", "--force", new_git_tag])


if __name__ == "__main__":
    main()
