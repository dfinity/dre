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


def check_if_should_pop_latest_rc(rc_name: str, index):
    output = subprocess.run(["git", "diff", "main", "--", "release-index.yaml"], capture_output=True, text=True)
    if output.returncode != 0:
        raise ValueError(f"Unexpected response from command: \n{output.stderr.strip()}")
    diff = output.stdout.strip()
    if not diff or rc_name in diff:
        return

    index["releases"].pop(0)


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
        check_if_should_pop_latest_rc(rc_name, index)
    except Exception as e:
        print(f"Error: {e}")
        exit(1)

    releases = index["releases"]
    elem_to_add = {"name": tag, "version": args.commit}
    elem_to_add = ruamel.yaml.CommentedMap(elem_to_add)
    elem_to_add.yaml_set_start_comment(f"Qualification pipeline:\n# {args.link}", indent=6)

    potential_release = list(filter(lambda release: release["rc_name"] == rc_name, releases))
    if len(potential_release) > 0:
        potential_release = potential_release[0]
        potential_release["versions"].append(elem_to_add)
    else:
        elem_to_add = ruamel.yaml.CommentedMap({"rc_name": rc_name, "versions": [elem_to_add]})
        releases.insert(0, elem_to_add)

    yaml.dump(index, open(index_path, "w"))


if __name__ == "__main__":
    main()
