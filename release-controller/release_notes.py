#!/usr/bin/env python3
import argparse
import fnmatch
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile
import time
import typing
from dataclasses import dataclass
from git_repo import GitRepo

import markdown

COMMIT_HASH_LENGTH = 9

REPLICA_TEAMS = set(
    [
        "consensus-owners",
        "consensus",
        "crypto-team",
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
    teams: typing.List[str]
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
    "crypto-team": "Crypto",
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
    "pocket-ic": "Pocket IC",
}


EXCLUDE_PACKAGES_FILTERS = [
    r".+\/sns\/.+",
    r".+\/ckbtc\/.+",
    r".+\/cketh\/.+",
    r"rs\/nns.+",
    r".+test.+",
    r"^bazel$",
    r".*boundary.*",
    r".*rosetta.*",
]

NON_REPLICA_TEAMS = sorted(list(set(TEAM_PRETTY_MAP.keys()) - REPLICA_TEAMS))

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


def matched_patterns(file_path, patterns):
    matches = [(p, fnmatch.fnmatch(file_path, p)) for p in patterns]
    matches = [match for match in matches if match[1]]
    if len(matches) == 0:
        return None
    matches = list(reversed([match[0] for match in matches]))
    return matches[0]


def prepare_release_notes(
    base_release_tag,
    base_release_commit,
    release_tag,
    release_commit,
    max_commits=1000,
) -> str:
    change_infos: dict[str, list[Change]] = {}

    ci_patterns = ["/**/*.lock", "/**/*.bzl"]

    ic_repo = GitRepo("https://github.com/dfinity/ic.git", main_branch="master")

    commits = ic_repo.get_commits_in_range(base_release_commit, release_commit)
    codeowners = parse_codeowners(ic_repo.file(".github/CODEOWNERS"))

    if len(commits) >= max_commits:
        print("WARNING: max commits limit reached, increase depth")
        exit(1)

    guestos_packages_all, guestos_packages_filtered = get_guestos_packages_with_bazel(ic_repo)

    for i, _ in progressbar([i[0] for i in commits], "Processing commit: ", 80):
        change_info = get_change_description_for_commit(
            commit_info=commits[i],
            change_infos=change_infos,
            ci_patterns=ci_patterns,
            ic_repo=ic_repo,
            codeowners=codeowners,
            guestos_packages_all=guestos_packages_all,
            guestos_packages_filtered=guestos_packages_filtered,
        )
        if change_info is None:
            continue

        commit_type = change_info["type"]
        change_infos[commit_type].append(change_info)

    return release_notes_markdown(
        ic_repo, base_release_tag, base_release_commit, release_tag, release_commit, change_infos
    )


def get_change_description_for_commit(
    commit_info,
    change_infos,
    ci_patterns,
    ic_repo,
    codeowners,
    guestos_packages_all,
    guestos_packages_filtered,
) -> typing.Optional[Change]:
    # Conventional commit regex pattern
    conv_commit_pattern = re.compile(r"^(\w+)(\([^\)]*\))?: (.+)$")
    # Jira ticket: <whitespace?><word boundary><uppercase letters><digit?><hyphen><digits><word boundary><colon?>
    jira_ticket_regex = r" *\b[A-Z]{2,}\d?-\d+\b:?"
    # Sometimes Jira tickets are in square brackets
    empty_brackets_regex = r" *\[ *\]:?"

    commit_hash, commit_message, commiter = commit_info

    file_changes = ic_repo.file_changes_for_commit(commit_hash)
    guestos_change = any(any(c["file_path"][1:].startswith(p) for c in file_changes) for p in guestos_packages_all)
    if not guestos_change:
        return None

    included = any(any(c["file_path"][1:].startswith(p) for c in file_changes) for p in guestos_packages_filtered)

    ownership = {}
    stripped_message = re.sub(jira_ticket_regex, "", commit_message)
    stripped_message = re.sub(empty_brackets_regex, "", stripped_message)
    # add github PR links
    stripped_message = re.sub(r"\(#(\d+)\)", r"([#\1](https://github.com/dfinity/ic/pull/\1))", stripped_message)
    stripped_message = stripped_message.strip()

    conventional = parse_conventional_commit(stripped_message, conv_commit_pattern)

    for change in file_changes:
        if any([fnmatch.fnmatch(change["file_path"], pattern) for pattern in ci_patterns]):
            continue

        teams = set(sum([codeowners[p] for p in codeowners.keys() if fnmatch.fnmatch(change["file_path"], p)], []))
        if not teams:
            teams = ["unknown"]

        for team in teams:
            if team not in ownership:
                ownership[team] = change["num_changes"]
                continue
            ownership[team] += change["num_changes"]

    if "ic-owners-owners" in ownership:
        ownership.pop("ic-owners-owners")

    # TODO: count max first by replica team then others
    teams = set()
    if ownership:
        replica_ownership = {team: lines for team, lines in ownership.items() if team in REPLICA_TEAMS}
        max_ownership_replica = max([lines for team, lines in ownership.items() if team in REPLICA_TEAMS] or [0])
        for key, value in replica_ownership.items():
            if value >= max_ownership_replica * MAX_OWNERSHIP_AREA:
                teams.add(key)
        if not teams:
            max_ownership = max(ownership.values() or [0])
            for key, value in ownership.items():
                if value >= max_ownership * MAX_OWNERSHIP_AREA:
                    teams.add(key)

    commit_type = conventional["type"].lower()
    commit_type = commit_type if commit_type in TYPE_PRETTY_MAP else "other"

    if not REPLICA_TEAMS.intersection(teams):
        included = False

    teams = sorted(list(teams))

    if commit_type not in change_infos:
        change_infos[commit_type] = []

    commiter_parts = commiter.split()
    commiter = "{:<4} {:<4}".format(
        commiter_parts[0][:4],
        commiter_parts[1][:4] if len(commiter_parts) >= 2 else "",
    )

    return Change(
        commit=commit_hash,
        teams=list(teams),
        type=commit_type,
        scope=conventional["scope"] if conventional["scope"] else "",
        message=conventional["message"],
        commiter=commiter,
        included=included,
    )


def release_notes_html(notes_markdown):
    """Generate release notes in HTML format, typically for local testing."""
    import webbrowser

    with tempfile.NamedTemporaryFile(suffix=".html", delete=False) as output:
        output.write(str.encode(markdown.markdown(notes_markdown)))
        filename = "file://{}".format(output.name)
        webbrowser.open_new_tab(filename)


def release_notes_markdown(
    ic_repo: GitRepo, base_release_tag, base_release_commit, release_tag, release_commit, change_infos
):
    """Generate release notes in markdown format."""
    merge_base = ic_repo.merge_base(base_release_tag, release_tag)

    reviewers_text = "\n".join([f"- {t.google_docs_handle}" for t in RELEASE_NOTES_REVIEWERS if t.send_announcement])

    notes = """\
# Review checklist

<span style="color: red">Please cross-out your team once you finished the review</span>

{reviewers_text}

# Release Notes for [{release_tag}](https://github.com/dfinity/ic/tree/{release_tag}) (`{release_commit}`)
This release is based on changes since [{base_release_tag}](https://dashboard.internetcomputer.org/release/{base_release_commit}) (`{base_release_commit}`).

Please note that some commits may be excluded from this release if they're not relevant, or not modifying the GuestOS image.
Additionally, descriptions of some changes might have been slightly modified to fit the release notes format.

To see a full list of commits added since last release, compare the revisions on [GitHub](https://github.com/dfinity/ic/compare/{base_release_tag}...{release_tag}).
""".format(
        base_release_tag=base_release_tag,
        base_release_commit=base_release_commit,
        release_tag=release_tag,
        release_commit=release_commit,
        reviewers_text=reviewers_text,
    )
    if merge_base != base_release_commit:
        notes += """
This release diverges from latest release. Merge base is [{merge_base}](https://github.com/dfinity/ic/tree/{merge_base}).
Changes [were removed](https://github.com/dfinity/ic/compare/{release_tag}...{base_release_tag}) from this release.
""".format(
            merge_base=merge_base,
            release_tag=release_tag,
            base_release_tag=base_release_tag,
        )

    for current_type in sorted(TYPE_PRETTY_MAP, key=lambda x: TYPE_PRETTY_MAP[x][1]):
        if current_type not in change_infos:
            continue
        notes += "## {0}:\n".format(TYPE_PRETTY_MAP[current_type][0])

        for change in sorted(change_infos[current_type], key=lambda x: ",".join(x["teams"])):
            commit_part = "[`{0}`](https://github.com/dfinity/ic/commit/{0})".format(change["commit"][:9])
            team_part = ",".join([TEAM_PRETTY_MAP.get(team, team) for team in change["teams"]])
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


def bazel_query_guestos_packages(ic_repo: GitRepo):
    """Bazel query package for GuestOS."""
    bazel_query = [
        "bazel",
        "query",
        "--universe_scope=//...",
        "deps(//ic-os/guestos/envs/prod:update-img.tar.gz) union deps(//ic-os/setupos/envs/prod:disk-img.tar.gz)",
        "--output=package",
    ]
    p = subprocess.run(
        ["gitlab-ci/container/container-run.sh"] + bazel_query,
        cwd=ic_repo.dir,
        text=True,
        stdout=subprocess.PIPE,
        check=False,
    )
    if p.returncode != 0:
        print("Failure running Bazel through container. Attempting direct run.", file=sys.stderr)
        p = subprocess.run(
            bazel_query,
            cwd=ic_repo.dir,
            text=True,
            stdout=subprocess.PIPE,
            check=True,
        )
    return p.stdout.strip().splitlines()


def get_guestos_packages_with_bazel(ic_repo: GitRepo):
    """Get the packages that are related to the GuestOS image using Bazel."""
    guestos_packages_all = bazel_query_guestos_packages(ic_repo)
    guestos_packages_filtered = [
        p for p in guestos_packages_all if not any(re.match(f, p) for f in EXCLUDE_PACKAGES_FILTERS)
    ]

    return guestos_packages_all, guestos_packages_filtered


def main():
    parser = argparse.ArgumentParser(description="Generate release notes")
    parser.add_argument("base_release_tag", type=str, help="base release tag")
    parser.add_argument("base_release_commit", type=str, help="base release commit")
    parser.add_argument("release_tag", type=str, help="release tag")
    parser.add_argument("release_commit", type=str, help="release commit")
    parser.add_argument(
        "--max-commits",
        default=os.environ.get("MAX_COMMITS", 1000),
        help="Maximum number of commits to include in the release notes",
    )
    args = parser.parse_args()

    release_notes_html(
        prepare_release_notes(
            args.base_release_tag,
            args.base_release_commit,
            args.release_tag,
            args.release_commit,
            max_commits=args.max_commits,
        )
    )


if __name__ == "__main__":
    main()
