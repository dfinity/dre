import argparse
import re
import subprocess

import ruamel.yaml
from ruamel.yaml.main import YAML


def get_toplevel() -> str:
    return subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True, check=True
    ).stdout.strip()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser("Automatic update of release index")
    parser.add_argument("commit", help="Commit of the release, it will be used to determine the rc")
    parser.add_argument("link", help="Link to the pipeline of qualification job")
    return parser.parse_args()


def get_branch_with_commit(commit: str) -> str:
    output = subprocess.run(
        ["git", "ls-remote", "--branches", "https://github.com/dfinity/ic.git"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()

    branch = list(filter(lambda line: line.startswith(commit), output.splitlines()))[0].split("/")[-1]
    return branch


def parse_branch(branch: str) -> tuple[str, str]:
    pattern = re.compile(r"^(rc--\d{4}-\d{2}-\d{2}_\d{2}-\d{2})(?:-(.*))?$")
    matches = pattern.match(branch)
    if not matches:
        raise ValueError("Input branch is not in the correct format `rc--%Y-%m-%d_%H-%M(-feature)")

    name = matches.group(1)
    feature = matches.group(2)
    if not feature:
        feature = "base"

    return (name, feature)


def pop_rcs_not_found_on_main(rc_name: str, index, yaml: YAML):
    # Remove all rcs not found on main unless its the same release as the potential added one.
    #
    # This can happen if the rc that is being added has a base and feature versions
    output = subprocess.run(["git", "show", "main:release-index.yaml"], capture_output=True, text=True)
    if output.returncode != 0:
        raise ValueError(f"Unexpected response from git: \n{output.stderr.strip()}")
    index_on_main = output.stdout.strip()
    index_on_main = yaml.load(index_on_main)
    rcs_on_main = [rc["rc_name"] for rc in index_on_main["releases"]]
    index["releases"] = [rc for rc in index["releases"] if rc["rc_name"] in rcs_on_main or rc["rc_name"] == rc_name]


def main():
    args = parse_args()

    try:
        branch = get_branch_with_commit(args.commit)
        print(f"Found branch: {branch}")
    except Exception as e:
        print(f"Didn't find branch with head {args.commit}, Error: {e}")
        exit(1)

    try:
        (rc_name, tag) = parse_branch(branch)
    except ValueError as e:
        print(e)
        exit(1)

    index_path = f"{get_toplevel()}/release-index.yaml"
    yaml = YAML(typ="rt")
    yaml.indent(mapping=4, sequence=4, offset=2)
    index = yaml.load(open(index_path, "r").read())
    try:
        pop_rcs_not_found_on_main(rc_name, index, yaml)
    except Exception as e:
        print(f"Error: {e}")
        exit(1)

    releases = index["releases"]
    elem_to_add = {"name": tag, "version": args.commit}
    elem_to_add = ruamel.yaml.CommentedMap(elem_to_add)
    elem_to_add.yaml_set_start_comment(f"Qualification pipeline:\n# {args.link}", indent=6)

    potential_release = next(filter(lambda release: release["rc_name"] == rc_name, releases), None)
    if not potential_release:
        potential_release = ruamel.yaml.CommentedMap({"rc_name": rc_name, "versions": []})
        releases.insert(0, potential_release)

    potential_release["versions"].append(elem_to_add)
    yaml.dump(index, open(index_path, "w"))


if __name__ == "__main__":
    main()
