#!/usr/bin/env python3
import argparse
import fnmatch
import os
import pathlib
import re
import subprocess
import sys
import time
import webbrowser


REPLICA_TEAMS = set(
    [
        "consensus-owners",
        "crypto-owners",
        "Orchestrator",
        "message-routing-owners",
        "networking-team",
        "execution-owners",
        "node-team",
        "runtime-owners",
    ]
)

TYPE_PRETTY_MAP = {
    "feat": ("Features", 0),
    "fix": ("Bugfixes", 1),
    "docs": ("Documentation", 6),
    "refactor": ("Refactoring", 4),
    "perf": ("Performance improvements", 2),
    "test": ("Tests", 5),
    "chore": ("Chores", 3),
    "other": ("Other changes", 7),
    "excluded": ("Excluded changes", 8),
}

TEAM_PRETTY_MAP = {
    "DRE": "DRE",
    "boundarynode-team": "Boundary Nodes",
    "chross-chain-team": "Cross Chain",
    "consensus-owners": "Consensus",
    "cross-chain-team": "Cross Chain",
    "crypto-owners": "Crypto",
    "docs-owners": "Docs",
    "execution-owners": "Execution",
    "financial-integrations": "Financial Integrations",
    "ghost": "Ghost",
    "ic-support-eu": "SupportEU",
    "ic-support-na": "SupportNA",
    "ic-testing-verification": "T&V",
    "idx": "IDX",
    "interface-owners": "Interface",
    "message-routing-owners": "Message Routing",
    "networking-team": "Networking",
    "nns-team": "NNS",
    "node-team": "Node",
    "owners-owners": "Owners",
    "platform-operations": "PfOps",
    "prodsec": "Prodsec",
    "runtime-owners": "Runtime",
    "trust-team": "Trust",
    "sdk-team": "SDK",
}


EXCLUDE_PACKAGES_FILTERS = [
    r".+\/sns\/.+",
    r".+\/ckbtc\/.+",
    r".+\/cketh\/.+",
    r".+canister.+",
    r"rs\/nns.+",
    r".+test.+",
    r"^bazel$",
]

EXCLUDED_TEAMS = set(TEAM_PRETTY_MAP.keys()) - REPLICA_TEAMS

# Ownership threshold for analyzing which teams were
# involved in the commit
MAX_OWNERSHIP_AREA = 0.5

parser = argparse.ArgumentParser(description="Generate release notes")
parser.add_argument("first_commit", type=str, help="first commit")
parser.add_argument("--last-commit", type=str, help="last commit", dest="last_commit", default="")
parser.add_argument(
    "--html",
    type=str,
    dest="html_path",
    default="$HOME/Downloads/release-notes.html",
    help="path to where the output should be generated",
)
args = parser.parse_args()


# https://stackoverflow.com/a/34482761
def progressbar(it, prefix="", size=60, out=sys.stdout):  # Python3.6+
    count = len(it)
    start = time.time()

    def show(j, item):
        x = int(size * j / count)
        remaining = ((time.time() - start) / j) * (count - j)

        mins, sec = divmod(remaining, 60)
        time_str = f"{int(mins):02}:{sec:05.2f}"

        print(
            f"{prefix}{item} [{'█'*x}{('.'*(size-x))}] {j}/{count} Est wait {time_str}",
            end="\r",
            file=out,
            flush=True,
        )

    for i, item in enumerate(it):
        yield i, item
        show(i + 1, item)
    print("\n", flush=True, file=out)


def get_commits(repo_dir, first_commit, last_commit):
    def get_commits_info(git_commit_format):
        return (
            subprocess.check_output(
                [
                    "git",
                    "--git-dir={}/.git".format(repo_dir),
                    "log",
                    "--format={}".format(git_commit_format),
                    "--no-merges",
                    "{}..{}".format(first_commit, last_commit),
                ],
                stderr=subprocess.DEVNULL,
            )
            .decode("utf-8")
            .strip()
            .split("\n")
        )

    commit_hashes = get_commits_info("%h")
    commit_messages = get_commits_info("%s")
    commiters = get_commits_info("%an")

    return list(zip(commit_hashes, commit_messages, commiters))


def file_changes_for_commit(commit_hash, repo_dir):
    cmd = [
        "git",
        "diff",
        "--numstat",
        f"{commit_hash}^..{commit_hash}",
    ]
    diffstat_output = (
        subprocess.check_output(
            cmd,
            cwd=repo_dir,
            stderr=subprocess.DEVNULL,
        )
        .decode()
        .strip()
    )

    parts = diffstat_output.splitlines()
    changes = []
    for line in parts:
        file_path = line.split()[2].strip()
        additions = line.split()[0].strip()
        deletions = line.split()[1].strip()
        additions = additions if additions != "-" else "0"
        deletions = deletions if deletions != "-" else "0"

        changes.append(
            {
                "file_path": "/" + file_path,
                "num_changes": int(additions) + int(deletions),
            }
        )

    return changes


def parse_codeowners(codeowners_path):
    with open(codeowners_path, encoding="utf8") as f:
        codeowners = f.readlines()
        filtered = [line.strip() for line in codeowners]
        filtered = [line for line in filtered if line and not line.startswith("#")]
        parsed = {}
        for line in filtered:
            result = line.split()
            teams = [team.split("@dfinity-lab/teams/")[1] for team in result[1:]]
            pattern = result[0]
            pattern = pattern if pattern.startswith("/") else "/" + pattern
            pattern = pattern if not pattern.endswith("/") else pattern + "*"

            parsed[pattern] = teams

        return parsed


def parse_conventional_commit(message, pattern):
    match = pattern.match(message)

    if match:
        commit_type = match.group(1)
        commit_scope = match.group(2)[1:-1] if match.group(2) else None
        commit_message = match.group(3)
        return {"type": commit_type, "scope": commit_scope, "message": commit_message}
    return {"type": "other", "scope": None, "message": message}


def best_matching_regex(file_path, regex_list):
    matches = [(regex, fnmatch.fnmatch(file_path, regex)) for regex in regex_list]
    matches = [match for match in matches if match[1]]
    if len(matches) == 0:
        return None
    matches = list(reversed([match[0] for match in matches]))
    return matches[0]


def strip_ansi_sequences(input_text):
    # https://stackoverflow.com/a/14693789
    ansi_escape = re.compile(r"\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])")
    return ansi_escape.sub("", input_text)


def run_bazel_query(query, ic_repo_path):
    p = subprocess.run(
        ["gitlab-ci/container/container-run.sh"] + query,
        cwd=ic_repo_path,
        text=True,
        stdout=subprocess.PIPE,
        check=False,
    )
    if p.returncode != 0:
        print("Failure running Bazel through container.  Attempting direct run.", file=sys.stderr)
        p = subprocess.run(
            query,
            cwd=ic_repo_path,
            text=True,
            stdout=subprocess.PIPE,
            check=True,
        )
    return [p.strip() for p in strip_ansi_sequences(p.stdout).strip().splitlines() if p.strip()]


def main():
    first_commit = args.first_commit
    last_commit = args.last_commit
    html_path = os.path.expandvars(args.html_path)
    conv_commit_pattern = re.compile(r"^(\w+)(\([^\)]*\))?: (.+)$")
    jira_ticket_regex = r" *\b[A-Z]{2,}\d?-\d+\b:?"  # <whitespace?><word boundary><uppercase letters><digit?><hyphen><digits><word boundary><colon?>
    empty_brackets_regex = r" *\[ *\]:?"  # Sometimes Jira tickets are in square brackets

    change_infos = {}

    ci_patterns = ["/**/*.lock", "/**/*.bzl"]

    ic_repo_path = pathlib.Path.home() / ".cache/git/ic"

    if ic_repo_path.exists():
        print("Fetching new commits in {}".format(ic_repo_path))
        subprocess.check_call(
            ["git", "fetch"],
            cwd=ic_repo_path,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        print("Resetting HEAD to latest origin/master.")
        subprocess.check_call(
            ["git", "reset", "--hard", "origin/master"],
            cwd=ic_repo_path,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    else:
        print("Cloning IC repo to {}".format(ic_repo_path))
        subprocess.check_call(
            [
                "git",
                "clone",
                "https://github.com/dfinity/ic.git",
                ic_repo_path,
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

    if last_commit == "":
        last_commit = subprocess.run(
            [
                "git",
                "rev-parse",
                "HEAD",
            ],
            cwd=ic_repo_path,
            text=True,
            check=True,
            stdout=subprocess.PIPE,
        ).stdout.strip()
        print("Last commit not set, using latest: {}".format(last_commit))

    codeowners = parse_codeowners(ic_repo_path / ".gitlab" / "CODEOWNERS")

    commits = get_commits(ic_repo_path, first_commit, last_commit)
    for i in range(len(commits)):
        commits[i] = commits[i] + (str(commits[i][0]),)

    print("Generating changes for hostos from {} commits".format(len(commits)))

    # Find all the packages that are relevant to the HostOS update image
    bazel_query = [
        "bazel",
        "query",
        "deps(//ic-os/hostos/envs/prod:update-img.tar.gz)",
        "--output=package",
    ]

    relevant_packages = run_bazel_query(bazel_query, ic_repo_path)
    relevant_paths = [p for p in relevant_packages if not p.startswith("@")]

    for i, _ in progressbar([i[0] for i in commits], "Processing commit: ", 80):
        commit_info = commits[i]
        commit_hash, commit_message, commiter, merge_commit = commit_info

        file_changes = file_changes_for_commit(commit_hash, ic_repo_path)
        changed_paths = set()
        for p in relevant_paths:
            for c in file_changes:
                changed_prefix = c["file_path"][1:]
                if changed_prefix.startswith(p):
                    changed_paths.add(p)
        if not changed_paths:
            continue

        ownership = {}
        stripped_message = re.sub(jira_ticket_regex, "", commit_message)
        stripped_message = re.sub(empty_brackets_regex, "", stripped_message)
        stripped_message = stripped_message.strip()

        conventional = parse_conventional_commit(stripped_message, conv_commit_pattern)

        for change in file_changes:
            if any([fnmatch.fnmatch(change["file_path"], pattern) for pattern in ci_patterns]):
                continue

            key = best_matching_regex(change["file_path"], codeowners.keys())
            teams = ["unknown"] if key is None else codeowners[key]

            for team in teams:
                if team not in ownership:
                    ownership[team] = change["num_changes"]
                    continue
                ownership[team] += change["num_changes"]

        # Non reviewed files
        if "ghost" in ownership:
            ownership.pop("ghost")
        if "owners-owners" in ownership:
            ownership.pop("owners-owners")

        teams = []
        if ownership:
            max_ownership = max(ownership.items(), key=lambda changed_lines_per_team: changed_lines_per_team[1])[1]
            # Since multiple teams can own a path in CODEOWNERS we have to handle what happens if two teams have max changes
            for key, value in ownership.items():
                if value >= max_ownership * MAX_OWNERSHIP_AREA:
                    teams.append(key)

            if "test" in conventional["message"]:
                conventional["type"] = "test"

        commit_type = conventional["type"].lower()
        commit_type = commit_type if commit_type in TYPE_PRETTY_MAP else "other"
        if len(teams) >= 3:
            # The change seems to be touching many teams, let's mark it as "other" (generic)
            commit_type = "other"

        included = True
        if ["ic-testing-verification"] == teams or all([team in EXCLUDED_TEAMS for team in teams]):
            included = False

        if commit_type not in change_infos:
            change_infos[commit_type] = []

        commiter_parts = commiter.split()
        commiter = "{:<4} {:<4}".format(
            commiter_parts[0][:4],
            commiter_parts[1][:4] if len(commiter_parts) >= 2 else "",
        )

        change_infos[commit_type].append(
            {
                "commit": merge_commit,
                "team": teams,
                "type": commit_type,
                "scope": conventional["scope"] if conventional["scope"] else "",
                "message": conventional["message"],
                "commiter": commiter,
                "included": included,
                "changed_path": ":".join(changed_paths),
            }
        )

    with open(html_path, "w", encoding="utf-8") as output:
        output.write(
            """
        <link rel="preconnect" href="https://fonts.googleapis.com">
        <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
        <link href="https://fonts.googleapis.com/css2?family=Roboto+Mono&display=swap" rel="stylesheet">
        """
        )
        output.write('<div style="font-family: Courier New; font-size: 8pt">')
        output.write(
            '<h1 id="{0}" style="font-size: 14pt; font-family: Roboto">Release Notes for HostOS <span style="font-family: Roboto Mono; font-weight: normal; font-size: 10pt">({0})</span></h1>\n'.format(
                last_commit
            )
        )
        output.write(
            "<br><p>Change log since git revision [{0}](https://github.com/dfinity/ic/commit/{0})</p>\n".format(
                first_commit
            )
        )

        for current_type in sorted(TYPE_PRETTY_MAP, key=lambda x: TYPE_PRETTY_MAP[x][1]):
            if current_type not in change_infos:
                continue
            output.write(
                '<h3 style="font-size: 14pt; font-family: Roboto">## {0}:</h3>\n'.format(
                    TYPE_PRETTY_MAP[current_type][0]
                )
            )

            for change in sorted(change_infos[current_type], key=lambda x: ",".join(x["team"])):
                commit_part = '[<a href="https://github.com/dfinity/ic/commit/{0}">{0}</a>]'.format(
                    change["commit"][:9]
                )
                team_part = ",".join([TEAM_PRETTY_MAP[team] for team in change["team"]])
                team_part = team_part if team_part else "General"
                scope_part = (
                    ":"
                    if change["scope"] == "" or change["scope"].lower() == team_part.lower()
                    else "({0}):".format(change["scope"])
                )
                message_part = change["message"]
                commiter_part = f"&lt!-- {change['commiter']} {change['changed_path']} --&gt"

                text = "* {0} {4} {1}{2} {3}<br>".format(
                    commit_part, team_part, scope_part, message_part, commiter_part
                )
                if not change["included"]:
                    text = "<s>{}</s>".format(text)
                output.write("<p style='font-size: 8pt; padding: 0; margin: 0; white-space: pre;'>{}</p>".format(text))

        output.write("</div>")

    filename = "file:///{}".format(html_path)
    webbrowser.open_new_tab(filename)


if __name__ == "__main__":
    main()
