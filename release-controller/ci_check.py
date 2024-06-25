import argparse
import json
import os
import pathlib
from datetime import datetime

import yaml
from colorama import Fore
from git_repo import GitRepo
from jsonschema import validate
from release_index import ReleaseIndex
from release_index_loader import ReleaseLoader

BASE_VERSION_NAME = "base"


def parse_args():
    parser = argparse.ArgumentParser(description="Tool for checking release index")
    parser.add_argument("--path", type=str, dest="path", help="Path to the release index", default="release-index.yaml")
    parser.add_argument(
        "--schema-path",
        type=str,
        dest="schema_path",
        help="Path to the release index schema",
        default="release-index-schema.json",
    )
    parser.add_argument(
        "--repo-path", type=str, dest="repo_path", help="Path to the repo", default=os.environ.get("IC_REPO_PATH")
    )

    return parser.parse_args()


def success_print(message: str):
    print(f"{Fore.GREEN}{message}{Fore.RESET}")


def print_and_ret(message: str) -> str:
    print(message)
    return message


def error_print(message: str) -> str:
    return print_and_ret(f"{Fore.RED}{message}{Fore.RESET}")


def warn_print(message: str) -> str:
    return print_and_ret(f"{Fore.YELLOW}{message}{Fore.RESET}")


def validate_schema(index: dict, schema_path: str) -> list[str]:
    with open(schema_path, "r", encoding="utf8") as f:
        schema = json.load(f)
        try:
            validate(instance=index, schema=schema)
        except Exception as e:
            return [error_print(f"Schema validation failed: \n{e}")]

    success_print("Schema validation passed")
    return []


def check_if_commits_really_exist(index: ReleaseIndex, repo: GitRepo) -> list[str]:
    errors = []
    for release in index.releases:
        for version in release.versions:
            commit = repo.show(version.version)
            if commit is None:
                errors.append(error_print(f"Commit {version.version} does not exist"))

    if len(errors) == 0:
        success_print("All commits exist")
    return errors


def check_if_there_is_a_base_version(index: ReleaseIndex) -> list[str]:
    errors = []
    for release in index.releases:
        found = False
        for version in release.versions:
            if version.name == BASE_VERSION_NAME:
                found = True
                break
        if not found:
            errors.append(error_print(f"Release {release.rc_name} does not have a base version"))

    if len(errors) == 0:
        success_print("All releases have a base version")
    return errors


def check_unique_version_names_within_release(index: ReleaseIndex) -> list[str]:
    errors = []
    for release in index.releases:
        version_names = set()
        for version in release.versions:
            if version.name in version_names:
                errors.append(
                    error_print(
                        f"Version {version.name} in release {release.rc_name} has the same name as another version from the same release"
                    )
                )
            version_names.add(version.name)

    if len(errors) == 0:
        success_print("All version names are unique within the respective releases")
    return errors


def check_version_to_tags_consistency(index: ReleaseIndex, repo: GitRepo) -> list[str]:
    errors = []
    for release in index.releases:
        for version in release.versions:
            tag_name = f"release-{release.rc_name.removeprefix('rc--')}-{version.name}"
            tag = repo.show(tag_name)
            commit = repo.show(version.version)
            if tag is None:
                warn_print(f"Tag {tag_name} does not exist")
                continue
            if tag.sha != commit.sha:
                errors.append(error_print(f"Tag {tag_name} points to {tag.sha} not {commit.sha}"))

    if len(errors) == 0:
        success_print("Finished consistency check")
    return errors


def check_rc_order(index: ReleaseIndex) -> list[str]:
    errors = []
    date_format = "%Y-%m-%d_%H-%M"
    parsed = [
        {"name": release.rc_name, "date": datetime.strptime(release.rc_name.removeprefix("rc--"), date_format)}
        for release in index.releases
    ]

    for i in range(1, len(parsed)):
        if parsed[i]["date"] > parsed[i - 1]["date"]:
            errors.append(error_print(f"Release {parsed[i]['name']} is older than {parsed[i - 1]['name']}"))

    if len(errors) == 0:
        success_print("All RC's are ordered descending by date")
    return errors


def check_versions_on_specific_branches(index: ReleaseIndex, repo: GitRepo) -> list[str]:
    errors = []
    for release in index.releases:
        for version in release.versions:
            commit = repo.show(version.version)
            if commit is None:
                errors.append(error_print(f"Commit {version.version} does not exist"))
                continue
            if release.rc_name not in commit.branches:
                errors.append(
                    error_print(
                        f"Commit {version.version} is not on branch {release.rc_name}. Commit found on brances: {', '.join(commit.branches)}"
                    )
                )

    if len(errors) == 0:
        success_print("All versions are on the correct branches")
    return errors


if __name__ == "__main__":
    args = parse_args()
    print(
        "Checking release index at '%s' against schmea at '%s' and repo at '%s'"
        % (args.path, args.schema_path, args.repo_path)
    )
    index = yaml.load(open(args.path, "r", encoding="utf8"), Loader=yaml.FullLoader)

    errors = []

    errors.extend(validate_schema(index, args.schema_path))

    index = ReleaseLoader(pathlib.Path(args.path).parent).index().root

    errors.extend(check_if_there_is_a_base_version(index))
    errors.extend(check_unique_version_names_within_release(index))
    errors.extend(check_rc_order(index))

    repo = GitRepo(
        "https://github.com/dfinity/ic.git", repo_cache_dir=pathlib.Path(args.repo_path).parent, main_branch="master"
    )
    repo.fetch()
    repo.ensure_branches([release.rc_name for release in index.releases])

    errors.extend(check_if_commits_really_exist(index, repo))
    errors.extend(check_versions_on_specific_branches(index, repo))
    errors.extend(check_version_to_tags_consistency(index, repo))
    # Check that versions from a release cannot be removed if notes were published to the forum

    if len(errors) > 0:
        exit(1)
    success_print("All checks passed")
