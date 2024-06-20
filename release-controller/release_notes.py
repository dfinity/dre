#!/usr/bin/env python3
import fnmatch
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
        "crypto-owners",
        "interface-owners",
        "Orchestrator",
        "message-routing-owners",
        "networking-team",
        "execution-owners",
        "node-team",
        "runtime-owners",
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

EXCLUDED_TEAMS = set(TEAM_PRETTY_MAP.keys()) - REPLICA_TEAMS

# Ownership threshold for analyzing which teams were
# involved in the commit
MAX_OWNERSHIP_AREA = 0.5

max_commits = 1000
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


def get_ancestry_path(repo_dir, commit_hash, branch_name):
    return (
        subprocess.check_output(
            [
                "git",
                "--git-dir",
                "{}/.git".format(repo_dir),
                "rev-list",
                "{}..{}".format(commit_hash, branch_name),
                "--ancestry-path",
            ]
        )
        .decode("utf-8")
        .strip()
        .split("\n")
    )


def get_first_parent(repo_dir, commit_hash, branch_name):
    return (
        subprocess.check_output(
            [
                "git",
                "--git-dir",
                "{}/.git".format(repo_dir),
                "rev-list",
                "{}..{}".format(commit_hash, branch_name),
                "--first-parent",
            ]
        )
        .decode("utf-8")
        .strip()
        .split("\n")
    )


def get_rc_branch(repo_dir, commit_hash):
    """Get the branch name for a commit hash."""
    all_branches = (
        subprocess.check_output(
            [
                "git",
                "--git-dir",
                "{}/.git".format(repo_dir),
                "branch",
                "--contains",
                commit_hash,
                "--remote",
            ]
        )
        .decode("utf-8")
        .strip()
        .splitlines()
    )
    all_branches = [branch.strip() for branch in all_branches]
    rc_branches = [branch for branch in all_branches if branch.startswith("origin/rc--20")]
    if rc_branches:
        return rc_branches[0]
    return ""


def get_merge_commit(repo_dir, commit_hash):
    # Reference: https://www.30secondsofcode.org/git/s/find-merge-commit/
    rc_branch = get_rc_branch(repo_dir, commit_hash)
    relevant_commits = list(enumerate(get_ancestry_path(repo_dir, commit_hash, rc_branch))) + list(
        enumerate(get_first_parent(repo_dir, commit_hash, rc_branch))
    )
    relevant_commits = sorted(relevant_commits, key=lambda index_commit: index_commit[1])
    checked_commits = set()
    commits = []
    for index, commit in relevant_commits:
        if commit not in checked_commits:
            checked_commits.add(commit)
            commits.append((index, commit))

    relevant_commits = sorted(commits, key=lambda index_commit: index_commit[0])

    return relevant_commits[-1][1]


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


def release_notes(first_commit, last_commit, release_name) -> str:
    conv_commit_pattern = re.compile(r"^(\w+)(\([^\)]*\))?: (.+)$")
    jira_ticket_regex = r" *\b[A-Z]{2,}\d?-\d+\b:?"  # <whitespace?><word boundary><uppercase letters><digit?><hyphen><digits><word boundary><colon?>
    empty_brackets_regex = r" *\[ *\]:?"  # Sometimes Jira tickets are in square brackets

    change_infos: dict[str, list[Change]] = {}

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

    codeowners = parse_codeowners(ic_repo_path / ".gitlab" / "CODEOWNERS")

    commits = get_commits(ic_repo_path, first_commit, last_commit)
    for i in range(len(commits)):
        commit_hash = str(commits[i][0])
        merge_commit = get_merge_commit(ic_repo_path, commit_hash)
        used_commit = (merge_commit or commit_hash)[:COMMIT_HASH_LENGTH]
        print("Commit: {} ==> using commit: {}".format(commit_hash, used_commit))
        commits[i] = commits[i] + (used_commit,)

    if len(commits) == max_commits:
        print("WARNING: max commits limit reached, increase depth")
        exit(1)

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
    replica_packages = p.stdout.strip().splitlines()

    replica_packages_filtered = [
        p for p in replica_packages if not any(re.match(f, p) for f in EXCLUDE_PACKAGES_FILTERS)
    ]

    for i, _ in progressbar([i[0] for i in commits], "Processing commit: ", 80):
        commit_info = commits[i]
        commit_hash, commit_message, commiter, merge_commit = commit_info

        file_changes = file_changes_for_commit(commit_hash, ic_repo_path)
        replica_change = any(any(c["file_path"][1:].startswith(p) for c in file_changes) for p in replica_packages)
        if not replica_change:
            continue

        included = any(any(c["file_path"][1:].startswith(p) for c in file_changes) for p in replica_packages_filtered)

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
            }
        )

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
