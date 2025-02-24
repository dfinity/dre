#!/usr/bin/env python3
import argparse
import fnmatch
import os
import pathlib
import re
import subprocess
import tempfile
import textwrap
import typing

from dataclasses import dataclass

from const import GUESTOS_CHANGED_NOTES_NAMESPACE
from git_repo import GitRepo
from util import auto_progressbar_with_item_descriptions

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
        "ic-owners-owners",
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
    exclusion_reason: typing.Optional[str]
    guestos_change: bool


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


EXCLUDE_CHANGES_FILTERS = [
    r"sns",
    r"ckbtc",
    r"cketh",
    r"rs\/nns.+",
    r"test",
    r"^bazel",
    r"boundary",
    r"rosetta",
    r"pocket[_-]ic",
    r"^Cargo.lock$",
    r"registry\/admin",
    r"canister(?!_(state|manager|snapshot|sandbox))",
]

EXCLUDED_SCOPES = [
    "ic-admin",
    "nns",
    "sns",
    "PocketIC",
    "registry",
]

INCLUDE_CHANGES = ["bazel/external_crates.bzl"]

NON_REPLICA_TEAMS = sorted(list(set(TEAM_PRETTY_MAP.keys()) - REPLICA_TEAMS))

# Ownership threshold for analyzing which teams were
# involved in the commit
MAX_OWNERSHIP_AREA = 0.5

branch = "master"


def branch_strip_remote(branch: str) -> str:
    return branch.split("/", 1)[1]


def get_rc_branch(repo_dir: str, commit_hash: str) -> str:
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
    rc_branches = [
        branch
        for branch in all_branches
        if branch_strip_remote(branch).startswith("rc--20")
    ]
    if rc_branches:
        return rc_branches[0]
    return ""


def parse_codeowners(codeowners_path: str | pathlib.Path) -> dict[str, list[str]]:
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


class ConventionalCommit(typing.TypedDict):
    type: str
    scope: str | None
    message: str


def parse_conventional_commit(
    message: str, pattern: re.Pattern[str]
) -> ConventionalCommit:
    match = pattern.match(message)

    if match:
        commit_type = match.group(1)
        commit_scope = match.group(2)[1:-1] if match.group(2) else None
        commit_message = match.group(3)
        return {"type": commit_type, "scope": commit_scope, "message": commit_message}
    return {"type": "other", "scope": None, "message": message}


def matched_patterns(file_path: str, patterns: typing.Iterator[str]) -> str | None:
    matches = [
        match
        for match, did_match in [(p, fnmatch.fnmatch(file_path, p)) for p in patterns]
        if did_match
    ]
    return matches[-1] if matches else None


def release_changes(
    ic_repo: GitRepo,
    base_release_commit: str,
    release_commit: str,
    max_commits: int = 1000,
) -> dict[str, list[Change]]:
    changes: dict[str, list[Change]] = {}

    commits = ic_repo.get_commits_info("%h", base_release_commit, release_commit)
    assert isinstance(commits, list), "Commits is not a list: %r" % (commits,)

    if len(commits) >= max_commits:
        print("WARNING: max commits limit reached, increase depth")
        exit(1)

    for i, _ in enumerate(
        auto_progressbar_with_item_descriptions(
            [(i[:8], i) for i in commits], "Commit "
        )
    ):
        change = get_change_description_for_commit(
            commit_hash=commits[i],
            ic_repo=ic_repo,
        )
        if change is None:
            continue

        commit_type = change["type"]
        if commit_type not in changes:
            changes[commit_type] = []
        changes[commit_type].append(change)

    return changes


class ReleaseNotesRequest(object):
    def __init__(self, release_tag: str, release_commit: str):
        self.release_tag = release_tag
        self.release_commit = release_commit


class SecurityReleaseNotesRequest(ReleaseNotesRequest):
    pass


class OrdinaryReleaseNotesRequest(ReleaseNotesRequest):
    def __init__(
        self,
        release_tag: str,
        release_commit: str,
        base_release_tag: str,
        base_release_commit: str,
    ):
        super().__init__(release_tag, release_commit)
        self.base_release_tag = base_release_tag
        self.base_release_commit = base_release_commit


class PreparedReleaseNotes(str):
    pass


def prepare_release_notes(
    request: SecurityReleaseNotesRequest | OrdinaryReleaseNotesRequest,
    max_commits: int = 1000,
) -> PreparedReleaseNotes:
    if isinstance(request, SecurityReleaseNotesRequest):
        # Special case to avoid generation of any release notes in the case of security fixes.
        # It would be impossible anyway since policy prohibits it, and the repository containing
        # the fixes is private.
        return PreparedReleaseNotes(
            textwrap.dedent(
                f"""\
                # Release Notes for [{request.release_tag}](https://github.com/dfinity/ic/tree/{request.release_tag}) (`{request.release_commit}`)

                In accordance with the Security Patch Policy and Procedure that was adopted in
                [proposal 48792](https://dashboard.internetcomputer.org/proposal/48792),
                the source code that was used to build this release will be disclosed at the latest
                10 days after the fix is rolled out to all subnets.

                The community will then be able to retroactively verify the binaries that were rolled out.
                """
            )
        )

    ic_repo = GitRepo("https://github.com/dfinity/ic.git", main_branch="master")
    changes = release_changes(
        ic_repo,
        request.base_release_commit,
        request.release_commit,
        max_commits,
    )
    return PreparedReleaseNotes(
        release_notes_markdown(
            ic_repo,
            request.base_release_tag,
            request.base_release_commit,
            request.release_tag,
            request.release_commit,
            changes,
        )
    )


def get_change_description_for_commit(
    commit_hash: str,
    ic_repo: GitRepo,
) -> Change:
    # Conventional commit regex pattern
    conv_commit_pattern = re.compile(r"^(\w+)(\([^\)]*\))?: (.+)$")
    # Jira ticket: <whitespace?><word boundary><uppercase letters><digit?><hyphen><digits><word boundary><colon?>
    jira_ticket_regex = r" *\b[A-Z]{2,}\d?-\d+\b:?"
    # Sometimes Jira tickets are in square brackets
    empty_brackets_regex = r" *\[ *\]:?"

    commit_message = ic_repo.get_commit_info("%s", commit_hash)
    commiter = ic_repo.get_commit_info("%an", commit_hash)

    ic_repo.checkout(commit_hash)
    file_changes = ic_repo.file_changes_for_commit(commit_hash)
    exclusion_reason = None
    guestos_change = is_guestos_change(ic_repo, commit_hash)
    if (
        guestos_change
        and not exclusion_reason
        and not any(
            f
            for f in file_changes
            if not any(
                f["file_path"] not in INCLUDE_CHANGES
                and re.search(filter, f["file_path"])
                for filter in EXCLUDE_CHANGES_FILTERS
            )
        )
    ):
        exclusion_reason = "Changed files are excluded by file path filter"

    ownership = {}
    stripped_message = re.sub(jira_ticket_regex, "", commit_message)
    stripped_message = re.sub(empty_brackets_regex, "", stripped_message)
    # add github PR links
    stripped_message = re.sub(
        r"\(#(\d+)\)",
        r"([#\1](https://github.com/dfinity/ic/pull/\1))",
        stripped_message,
    )
    stripped_message = stripped_message.strip()

    conventional = parse_conventional_commit(stripped_message, conv_commit_pattern)

    codeowners = parse_codeowners(ic_repo.file(".github/CODEOWNERS"))
    for change in file_changes:
        teams = set(
            sum(
                [
                    codeowners[p]
                    for p in codeowners.keys()
                    if fnmatch.fnmatch(change["file_path"], p.removeprefix("/"))
                ],
                [],
            )
        )
        if not teams:
            teams = set(["unknown"])

        for team in teams:
            if team not in ownership:
                ownership[team] = change["num_changes"]
                continue
            ownership[team] += change["num_changes"]

    if (
        "ic-owners-owners" in ownership
        and len(set(ownership.keys()).intersection(REPLICA_TEAMS)) > 1
    ):
        ownership.pop("ic-owners-owners")

    # TODO: count max first by replica team then others
    teams = set()
    if ownership:
        replica_ownership = {
            team: lines for team, lines in ownership.items() if team in REPLICA_TEAMS
        }
        max_ownership_replica = max(
            [lines for team, lines in ownership.items() if team in REPLICA_TEAMS] or [0]
        )
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

    if (
        guestos_change
        and not exclusion_reason
        and not REPLICA_TEAMS.intersection(teams)
    ):
        exclusion_reason = "The change is not owned by any replica team"

    scope = conventional["scope"] if conventional["scope"] else ""
    if guestos_change and not exclusion_reason and scope in EXCLUDED_SCOPES:
        exclusion_reason = f"Scope of the change ({scope}) is not related to GuestOS"

    commiter_parts = commiter.split()
    commiter = "{:<4} {:<4}".format(
        commiter_parts[0][:4],
        commiter_parts[1][:4] if len(commiter_parts) >= 2 else "",
    )

    return Change(
        commit=commit_hash,
        teams=list(sorted(list(teams))),
        type=commit_type,
        scope=scope,
        message=conventional["message"],
        commiter=commiter,
        exclusion_reason=exclusion_reason,
        guestos_change=guestos_change,
    )


def release_notes_html(notes_markdown: str) -> None:
    """Generate release notes in HTML format, typically for local testing."""
    import webbrowser

    md = markdown.Markdown(
        extensions=["pymdownx.tilde", "pymdownx.details"],
    )

    with tempfile.NamedTemporaryFile(suffix=".html", delete=False) as output:
        output.write(str.encode(md.convert(notes_markdown)))
        filename = "file://{}".format(output.name)
        webbrowser.open_new_tab(filename)


def release_notes_markdown(
    ic_repo: GitRepo,
    base_release_tag: str,
    base_release_commit: str,
    release_tag: str,
    release_commit: str,
    change_infos: dict[str, list[Change]],
) -> str:
    """Generate release notes in markdown format."""
    merge_base = ic_repo.merge_base(base_release_commit, release_commit)

    reviewers_text = "\n".join(
        [
            f"- {t.google_docs_handle}"
            for t in RELEASE_NOTES_REVIEWERS
            if t.send_announcement
        ]
    )

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
This release diverges from the latest release. Merge base is [{merge_base}](https://github.com/dfinity/ic/tree/{merge_base}).
Changes [were removed](https://github.com/dfinity/ic/compare/{release_tag}...{base_release_tag}) from this release.
""".format(
            merge_base=merge_base,
            release_tag=release_tag,
            base_release_tag=base_release_tag,
        )

    def format_change(change: Change) -> str:
        commit_part = "[`{0}`](https://github.com/dfinity/ic/commit/{0})".format(
            change["commit"][:9]
        )
        team_part = ",".join(
            [TEAM_PRETTY_MAP.get(team, team) for team in change["teams"]]
        )
        team_part = team_part if team_part else "General"
        scope_part = (
            ":"
            if change["scope"] == "" or change["scope"].lower() == team_part.lower()
            else "({0}):".format(change["scope"])
        )
        message_part = change["message"]
        commiter_part = f"author: {change['commiter']}"

        text = "{4} | {0} {1}{2} {3}".format(
            commit_part, team_part, scope_part, message_part, commiter_part
        )
        if change["exclusion_reason"] or not change["guestos_change"]:
            text = "~~{} [AUTO-EXCLUDED:{}]~~".format(
                text,
                "Not modifying GuestOS"
                if not change["guestos_change"]
                else change["exclusion_reason"],
            )
        return "* " + text + "\n"

    non_guestos_changes = []
    for current_type in sorted(TYPE_PRETTY_MAP, key=lambda x: TYPE_PRETTY_MAP[x][1]):
        if current_type not in change_infos or not change_infos[current_type]:
            continue
        notes += "## {0}:\n".format(TYPE_PRETTY_MAP[current_type][0])

        for change in sorted(
            change_infos[current_type], key=lambda x: ",".join(x["teams"])
        ):
            if not change["guestos_change"]:
                non_guestos_changes.append(change)
                continue

            notes += format_change(change)

    if non_guestos_changes:
        notes += "## ~~Other changes not modifying GuestOS~~\n"
        for change in non_guestos_changes:
            notes += format_change(change)
    return notes


def is_guestos_change(ic_repo: GitRepo, commit: str) -> bool:
    """Check if GuestOS changed for the commit by querying git notes populated by commit annotator."""
    changed = ic_repo.get_note(GUESTOS_CHANGED_NOTES_NAMESPACE, commit)
    if not changed:
        raise ValueError(
            f"Could not find GuestOS label for commit {commit}. Check out commit annotator logs and runbook: https://dfinity.github.io/dre/release.html#missing-guestos-label."
        )
    changed = changed.strip()
    if changed == "True":
        return True
    elif changed == "False":
        return False
    else:
        raise ValueError(f"Invalid value for changed note {changed}")


def main() -> None:
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

    release_notes = prepare_release_notes(
        OrdinaryReleaseNotesRequest(
            args.release_tag,
            args.release_commit,
            args.base_release_tag,
            args.base_release_commit,
        ),
        max_commits=args.max_commits,
    )
    print(release_notes)
    release_notes_html(release_notes)


if __name__ == "__main__":
    main()
