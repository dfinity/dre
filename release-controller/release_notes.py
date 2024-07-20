#!/usr/bin/env python3
import argparse
import fnmatch
import json
import os
import pathlib
import re
import subprocess
import sys
import time
import typing
from dataclasses import dataclass

COMMIT_HASH_LENGTH = 9

REPLICA_TEAMS = set(
    [
        "consensus-owners",
        "consensus",
        "crypto-owners",
        "dept-crypto-library",
        "execution-owners",
        "execution",
        "ic-interface-owners",
        "ic-message-routing-owners",
        "interface-owners",
        "message-routing-owners",
        "networking-team",
        "networking",
        "node-team",
        "node",
        "Orchestrator",
        "runtime-owners",
        "runtime",
    ]
)


class Change(typing.TypedDict):
    """Change dataclass."""

    commit: str
    team: str
    type: str
    scope: str
    message: str
    commiter: str
    included: bool


@dataclass
class Team:
    """Team dataclass."""

    name: str
    google_docs_handle: str
    slack_id: str
    send_announcement: bool


RELEASE_NOTES_REVIEWERS = [
    Team("consensus", "@team-consensus", "SRJ3R849E", False),
    Team("crypto", "@team-crypto", "SU7BZQ78E", False),
    Team("execution", "@team-execution", "S01A577UL56", True),
    Team("messaging", "@team-messaging", "S01SVC713PS", True),
    Team("networking", "@team-networking", "SR6KC1DMZ", False),
    Team("node", "@node-team", "S027838EY30", False),
    Team("runtime", "@team-runtime", "S03BM6C0CJY", False),
]

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
    "boundary-node": "Boundary Nodes",
    "boundarynode-team": "Boundary Nodes",
    "consensus-owners": "Consensus",
    "consensus": "Consensus",
    "cross-chain-team": "Cross Chain",
    "crypto-owners": "Crypto",
    "dept-crypto-library": "Crypto",
    "docs-owners": "Docs",
    "dre": "DRE",
    "DRE": "DRE",
    "execution-owners": "Execution",
    "execution": "Execution",
    "financial-integrations": "Financial Integrations",
    "finint": "Financial Integrations",
    "ghost": "Ghost",
    "ic-interface-owners": "Interface",
    "ic-message-routing-owners": "Message Routing",
    "ic-owners-owners": "Owners",
    "ic-support-eu": "SupportEU",
    "ic-support-na": "SupportNA",
    "ic-support": "Support",
    "ic-testing-verification": "T&V",
    "idx": "IDX",
    "interface-owners": "Interface",
    "message-routing-owners": "Message Routing",
    "networking-team": "Networking",
    "networking": "Networking",
    "nns-team": "NNS",
    "node-team": "Node",
    "node": "Node",
    "owners-owners": "Owners",
    "platform-operations": "Platform Ops",
    "prodsec": "Prodsec",
    "product-security": "Prodsec",
    "runtime-owners": "Runtime",
    "runtime": "Runtime",
    "sdk-team": "SDK",
    "trust-team": "Trust",
    "utopia": "Utopia",
}


EXCLUDE_PACKAGES_FILTERS = [
    r".+\/sns\/.+",
    r".+\/ckbtc\/.+",
    r".+\/cketh\/.+",
    r"rs\/nns.+",
    r".+test.+",
    r"^bazel$",
]

NON_REPLICA_TEAMS = set(TEAM_PRETTY_MAP.keys()) - REPLICA_TEAMS

# Completely remove these teams from mentioning in the release notes
DROP_TEAMS = {"Utopia", "Financial Integrations", "IDX", "T&V", "Prodsec", "Support", "SupportEU", "SupportNA"}

# Ownership threshold for analyzing which teams were
# involved in the commit
MAX_OWNERSHIP_AREA = 0.5

branch = "master"


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


def branch_strip_remote(branch: str):
    return branch.split("/", 1)[1]


def get_rc_branch(repo_dir, commit_hash):
    """Get the branch name for a commit hash."""
    all_branches = (
        subprocess.check_output(
            [
                "git",
                "branch",
                "--contains",
                commit_hash,
                "--remote",
            ],
            cwd=repo_dir,
        )
        .decode("utf-8")
        .strip()
        .splitlines()
    )
    all_branches = [branch.strip() for branch in all_branches]
    rc_branches = [branch for branch in all_branches if branch_strip_remote(branch).startswith("rc--20")]
    if rc_branches:
        return rc_branches[0]
    return ""


def get_merge_commit(repo_dir, commit_hash):
    # Reference: https://stackoverflow.com/questions/8475448/find-merge-commit-which-includes-a-specific-commit
    rc_branch = get_rc_branch(repo_dir, commit_hash)

    try:
        # Run the Git commands and capture their output
        git_cmd = ["git", "rev-list", f"{commit_hash}..{rc_branch}"]
        ancestry_path = subprocess.run(
            git_cmd + ["--ancestry-path"],
            cwd=repo_dir,
            capture_output=True,
            text=True,
            check=True,
        ).stdout.splitlines()
        first_parent = subprocess.run(
            git_cmd + ["--first-parent"],
            cwd=repo_dir,
            capture_output=True,
            text=True,
            check=True,
        ).stdout.splitlines()

        # Combine and process the outputs
        combined_output = [(i + 1, line) for i, line in enumerate(ancestry_path + first_parent)]
        combined_output.sort(key=lambda x: x[1])

        # Find duplicates
        seen = {}
        duplicates = []
        for number, commit_hash in combined_output:
            if commit_hash in seen:
                duplicates.append((seen[commit_hash], number, commit_hash))
            seen[commit_hash] = number

        # Sort by the original line number and get the last one
        if duplicates:
            duplicates.sort()
            _, _, merge_commit = duplicates[-1]
            return merge_commit
        return None

    except subprocess.CalledProcessError as e:
        print(f"Error: {e}")
        return None


def get_commits(repo_dir, first_commit, last_commit):
    def get_commits_info(git_commit_format):
        return (
            subprocess.check_output(
                [
                    "git",
                    "log",
                    "--format={}".format(git_commit_format),
                    "--no-merges",
                    "{}..{}".format(first_commit, last_commit),
                ],
                cwd=repo_dir,
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
            teams = [team.split("@dfinity/")[1] for team in result[1:]]
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


def prepare_release_notes(first_commit, last_commit, release_name, max_commits=1000, write_to_html=None) -> str:
    change_infos: dict[str, list[Change]] = {}

    ci_patterns = ["/**/*.lock", "/**/*.bzl"]

    ic_repo_path = pathlib.Path.home() / ".cache/git/ic"
    commits = get_commits_in_range(ic_repo_path, first_commit, last_commit)
    codeowners = parse_codeowners(ic_repo_path / ".github" / "CODEOWNERS")

    if len(commits) >= max_commits:
        print("WARNING: max commits limit reached, increase depth")
        exit(1)

    guestos_packages_all, guestos_packages_filtered = get_guestos_packages_with_bazel(ic_repo_path)

    for i, _ in progressbar([i[0] for i in commits], "Processing commit: ", 80):
        change_info = get_change_description_for_commit(
            commit_info=commits[i],
            change_infos=change_infos,
            ci_patterns=ci_patterns,
            ic_repo_path=ic_repo_path,
            codeowners=codeowners,
            guestos_packages_all=guestos_packages_all,
            guestos_packages_filtered=guestos_packages_filtered,
        )
        if change_info is None:
            continue

        commit_type = change_info["type"]
        change_infos[commit_type].append(change_info)

    if write_to_html:
        release_notes_html(first_commit, last_commit, release_name, change_infos, write_to_html)
        return ""
    return release_notes_markdown(first_commit, last_commit, release_name, change_infos)


def get_change_description_for_commit(
    commit_info,
    change_infos,
    ci_patterns,
    ic_repo_path,
    codeowners,
    guestos_packages_all,
    guestos_packages_filtered,
):
    # Conventional commit regex pattern
    conv_commit_pattern = re.compile(r"^(\w+)(\([^\)]*\))?: (.+)$")
    # Jira ticket: <whitespace?><word boundary><uppercase letters><digit?><hyphen><digits><word boundary><colon?>
    jira_ticket_regex = r" *\b[A-Z]{2,}\d?-\d+\b:?"
    # Sometimes Jira tickets are in square brackets
    empty_brackets_regex = r" *\[ *\]:?"

    commit_hash, commit_message, commiter, merge_commit = commit_info

    file_changes = file_changes_for_commit(commit_hash, ic_repo_path)
    guestos_change = any(any(c["file_path"][1:].startswith(p) for c in file_changes) for p in guestos_packages_all)
    if not guestos_change:
        return None

    included = any(any(c["file_path"][1:].startswith(p) for c in file_changes) for p in guestos_packages_filtered)

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

    teams = set()
    if ownership:
        max_ownership = max(ownership.items(), key=lambda changed_lines_per_team: changed_lines_per_team[1])[1]
        # Since multiple teams can own a path in CODEOWNERS we have to handle what happens if two teams have max changes
        for key, value in ownership.items():
            if value >= max_ownership * MAX_OWNERSHIP_AREA:
                teams.add(key)

        if "test" in conventional["message"]:
            conventional["type"] = "test"

    commit_type = conventional["type"].lower()
    commit_type = commit_type if commit_type in TYPE_PRETTY_MAP else "other"

    teams = teams - DROP_TEAMS

    if not teams or all([team in NON_REPLICA_TEAMS for team in teams]):
        included = False

    if commit_type not in change_infos:
        change_infos[commit_type] = []

    commiter_parts = commiter.split()
    commiter = "{:<4} {:<4}".format(
        commiter_parts[0][:4],
        commiter_parts[1][:4] if len(commiter_parts) >= 2 else "",
    )

    change_info = {
        "commit": merge_commit,
        "team": list(teams),
        "type": commit_type,
        "scope": conventional["scope"] if conventional["scope"] else "",
        "message": conventional["message"],
        "commiter": commiter,
        "included": included,
    }

    return change_info


def get_commits_in_range(ic_repo_path, first_commit, last_commit):
    """Get the commits in the range [first_commit, last_commit] from the IC repo."""
    # Cache merge commits to avoid repeated slow calls to git
    merge_commits_cache_path = ic_repo_path / ".git/merge_commits.json"
    merge_commits_cache = {}
    if merge_commits_cache_path.exists():
        merge_commits_cache = json.loads(merge_commits_cache_path.read_text())

    if ic_repo_path.exists():
        print("Resetting HEAD to latest origin/master.")
        subprocess.check_call(
            ["git", "checkout", "--force", "master"],
            cwd=ic_repo_path,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        subprocess.check_call(
            ["git", "reset", "--hard", "origin/master"],
            cwd=ic_repo_path,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        print("Fetching new commits in {}".format(ic_repo_path))
        subprocess.check_call(
            ["git", "fetch"],
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

    commits = get_commits(ic_repo_path, first_commit, last_commit)
    for i in range(len(commits)):
        commit_hash = str(commits[i][0])
        if commit_hash in merge_commits_cache:
            merge_commit = merge_commits_cache[commit_hash]
        else:
            merge_commit = get_merge_commit(ic_repo_path, commit_hash)
            merge_commits_cache[commit_hash] = merge_commit
        used_commit = (merge_commit or commit_hash)[:COMMIT_HASH_LENGTH]
        print("Commit: {} ==> using commit: {}".format(commit_hash, used_commit))
        commits[i] = commits[i] + (used_commit,)

    merge_commits_cache_path.write_text(json.dumps(merge_commits_cache))

    return commits


def release_notes_html(first_commit, last_commit, release_name, change_infos, html_path):
    """Generate release notes in HTML format, typically for local testing."""
    import webbrowser

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
            '<h1 id="{0}" style="font-size: 14pt; font-family: Roboto">Release Notes for <a style="font-size: 10pt; font-family: Roboto Mono" href="https://github.com/dfinity/ic/tree/{0}">{0}</a> <span style="font-family: Roboto Mono; font-weight: normal; font-size: 10pt">({1})</span></h1>\n'.format(
                release_name, last_commit
            )
        )
        output.write(
            "<br><p>Change log since git revision [{0}](https://dashboard.internetcomputer.org/release/{0})</p>\n".format(
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
                    change["commit"][:COMMIT_HASH_LENGTH]
                )
                team_part = ",".join([TEAM_PRETTY_MAP.get(team, team) for team in change["team"]])
                team_part = team_part if team_part else "General"
                scope_part = (
                    ":"
                    if change["scope"] == "" or change["scope"].lower() == team_part.lower()
                    else "({0}):".format(change["scope"])
                )
                message_part = change["message"]
                commiter_part = f"&lt!-- {change['commiter']} --&gt"

                text = "* {0} {4} {1}{2} {3} {5}<br>".format(
                    commit_part,
                    team_part,
                    scope_part,
                    message_part,
                    commiter_part,
                    "" if change["included"] else "[AUTO-EXCLUDED]",
                )
                if not change["included"]:
                    text = "<s>{}</s>".format(text)
                output.write("<p style='font-size: 8pt; padding: 0; margin: 0; white-space: pre;'>{}</p>".format(text))

        output.write("</div>")

    html_path = os.path.abspath(html_path)

    filename = "file://{}".format(html_path)
    webbrowser.open_new_tab(filename)


def release_notes_markdown(first_commit, last_commit, release_name, change_infos):
    """Generate release notes in markdown format."""
    reviewers_text = "\n".join([f"- {t.google_docs_handle}" for t in RELEASE_NOTES_REVIEWERS if t.send_announcement])

    notes = """\
# Review checklist

<span style="color: red">Please cross-out your team once you finished the review</span>

{reviewers_text}

# Release Notes for [{rc_name}](https://github.com/dfinity/ic/tree/{rc_name}) ({last_commit})
Changelog since git revision [{first_commit}](https://dashboard.internetcomputer.org/release/{first_commit})
""".format(
        rc_name=release_name,
        last_commit=last_commit,
        first_commit=first_commit,
        reviewers_text=reviewers_text,
    )

    for current_type in sorted(TYPE_PRETTY_MAP, key=lambda x: TYPE_PRETTY_MAP[x][1]):
        if current_type not in change_infos:
            continue
        notes += "## {0}:\n".format(TYPE_PRETTY_MAP[current_type][0])

        for change in sorted(change_infos[current_type], key=lambda x: ",".join(x["team"])):
            commit_part = "[`{0}`](https://github.com/dfinity/ic/commit/{0})".format(change["commit"][:9])
            team_part = ",".join([TEAM_PRETTY_MAP.get(team, team) for team in change["team"]])
            team_part = team_part if team_part else "General"
            scope_part = (
                ":"
                if change["scope"] == "" or change["scope"].lower() == team_part.lower()
                else "({0}):".format(change["scope"])
            )
            message_part = change["message"]
            commiter_part = f"author: {change['commiter']}"

            text = "{4} | {0} {1}{2} {3}".format(commit_part, team_part, scope_part, message_part, commiter_part)
            if not change["included"]:
                text = "~~{} [AUTO-EXCLUDED]~~".format(text)
            notes += "* " + text + "\n"

    return notes


def get_guestos_packages_with_bazel(ic_repo_path):
    """Get the packages that are related to the GuestOS image using Bazel."""
    bazel_query = [
        "bazel",
        "query",
        "--universe_scope=//...",
        "deps(//ic-os/guestos/envs/prod:update-img.tar.gz) union deps(//ic-os/setupos/envs/prod:disk-img.tar.gz)",
        "--output=package",
    ]
    p = subprocess.run(
        ["gitlab-ci/container/container-run.sh"] + bazel_query,
        cwd=ic_repo_path,
        text=True,
        stdout=subprocess.PIPE,
        check=False,
    )
    if p.returncode != 0:
        print("Failure running Bazel through container.  Attempting direct run.", file=sys.stderr)
        p = subprocess.run(
            bazel_query,
            cwd=ic_repo_path,
            text=True,
            stdout=subprocess.PIPE,
            check=True,
        )
    guestos_packages_all = p.stdout.strip().splitlines()

    guestos_packages_filtered = [
        p for p in guestos_packages_all if not any(re.match(f, p) for f in EXCLUDE_PACKAGES_FILTERS)
    ]

    return guestos_packages_all, guestos_packages_filtered


def main():
    parser = argparse.ArgumentParser(description="Generate release notes")
    parser.add_argument("first_commit", type=str, help="first commit")
    parser.add_argument("last_commit", type=str, help="last commit")
    parser.add_argument(
        "--max-commits",
        default=os.environ.get("MAX_COMMITS", 1000),
        help="Maximum number of commits to include in the release notes",
    )
    parser.add_argument(
        "--html-path",
        type=str,
        default=None,
        help="Path of the output HTML file. Default is to generate a markdown file.",
    )
    parser.add_argument("rc_name", type=str, help="name of the release i.e. 'rc--2023-01-12_18-31'")
    args = parser.parse_args()

    print(prepare_release_notes(args.first_commit, args.last_commit, args.rc_name, args.max_commits, args.html_path))


if __name__ == "__main__":
    main()
