#!/usr/bin/env python3
import argparse
import fnmatch
import functools
import logging
import os
import pathlib
import re
import subprocess
import sys
import textwrap
import typing

from dataclasses import dataclass

mydir = os.path.join(os.path.dirname(__file__))
if mydir not in sys.path:
    sys.path.append(mydir)

from const import (  # noqa: E402
    OsKind,
    OS_KINDS,
    GUESTOS,
)
from commit_annotation import (  # noqa: E402
    RecreatingCommitChangeDeterminator,
    LocalCommitChangeDeterminator,
    CommitAnnotatorClientCommitChangeDeterminator,
    ChangeDeterminatorProtocol,
    COMMIT_BELONGS,
    COMMIT_COULD_NOT_BE_ANNOTATED,
)
from git_repo import GitRepo, FileChange  # noqa: E402
from util import auto_progressbar_with_item_descriptions, conventional_logging  # noqa: E402

import markdown  # noqa: E402

_LOGGER = logging.getLogger(__name__)

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
    belongs_to_this_release: bool


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


def parse_codeowners(codeowners_text: str) -> dict[str, list[str]]:
    codeowners = codeowners_text.splitlines(True)
    filtered = [line.strip() for line in codeowners]
    filtered = [line for line in filtered if line and not line.startswith("#")]
    parsed = {}
    for line in filtered:
        # _LOGGER.debug("Parsing CODEOWNERS, line: %s" % line)
        result = line.split()
        if len(result) <= 1:
            continue
        teams = [
            team.split("@dfinity/")[1] for team in result[1:] if "@dfinity/" in team
        ]
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
    belongs_determinator: ChangeDeterminatorProtocol,
    os_kind: OsKind,
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
            belongs=belongs_determinator.commit_changes_artifact(commits[i], os_kind)
            in [COMMIT_BELONGS, COMMIT_COULD_NOT_BE_ANNOTATED],
        )
        if change is None:
            continue

        commit_type = change["type"]
        if commit_type not in changes:
            changes[commit_type] = []
        changes[commit_type].append(change)

    return changes


class ReleaseNotesRequest(object):
    def __init__(self, release_tag: str, release_commit: str, os_kind: OsKind):
        self.release_tag = release_tag
        self.release_commit = release_commit
        self.os_kind = os_kind


class SecurityReleaseNotesRequest(ReleaseNotesRequest):
    pass


class OrdinaryReleaseNotesRequest(ReleaseNotesRequest):
    def __init__(
        self,
        release_tag: str,
        release_commit: str,
        base_release_tag: str,
        base_release_commit: str,
        os_kind: OsKind,
    ):
        super().__init__(release_tag, release_commit, os_kind)
        self.base_release_tag = base_release_tag
        self.base_release_commit = base_release_commit


class PreparedReleaseNotes(str):
    pass


def prepare_release_notes(
    request: SecurityReleaseNotesRequest | OrdinaryReleaseNotesRequest,
    ic_repo: GitRepo,
    os_change_determinator: ChangeDeterminatorProtocol,
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

    return PreparedReleaseNotes(
        release_notes_markdown(
            ic_repo,
            request.base_release_tag,
            request.base_release_commit,
            request.release_tag,
            request.release_commit,
            request.os_kind,
            release_changes(
                ic_repo,
                request.base_release_commit,
                request.release_commit,
                os_change_determinator,
                request.os_kind,
                max_commits,
            ),
        )
    )


def compose_change_description(
    commit_hash: str,
    commit_message: str,
    commiter: str,
    file_changes: list[FileChange],
    codeowners: dict[str, list[str]],
    belongs: bool,
) -> Change:
    # Conventional commit regex pattern
    conv_commit_pattern = re.compile(r"^(\w+)(\([^\)]*\))?: (.+)$")
    # Jira ticket: <whitespace?><word boundary><uppercase letters><digit?><hyphen><digits><word boundary><colon?>
    jira_ticket_regex = r" *\b[A-Z]{2,}\d?-\d+\b:?"
    # Sometimes Jira tickets are in square brackets
    empty_brackets_regex = r" *\[ *\]:?"

    exclusion_reason = None
    if (
        belongs
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

    if belongs and not exclusion_reason and not REPLICA_TEAMS.intersection(teams):
        exclusion_reason = "The change is not owned by any replica or HostOS team"

    scope = conventional["scope"] if conventional["scope"] else ""
    if belongs and not exclusion_reason and scope in EXCLUDED_SCOPES:
        exclusion_reason = (
            f"Scope of the change ({scope}) is not related to the artifact"
        )

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
        belongs_to_this_release=belongs,
    )


def get_change_description_for_commit(
    commit_hash: str,
    ic_repo: GitRepo,
    belongs: bool,
) -> Change:
    @functools.cache
    def parse_and_cache_codeowners(commit_id: str) -> dict[str, list[str]]:
        codeowners_text = ic_repo.file_contents(
            commit_hash,
            pathlib.Path(".github/CODEOWNERS"),
        ).decode("utf-8")
        return parse_codeowners(codeowners_text)

    commit_message = ic_repo.get_commit_info("%s", commit_hash)
    committer = ic_repo.get_commit_info("%an", commit_hash)
    file_changes_for_commit = ic_repo.file_changes_for_commit(commit_hash)
    codeowners = parse_and_cache_codeowners(commit_hash)

    return compose_change_description(
        commit_hash,
        commit_message,
        committer,
        file_changes_for_commit,
        codeowners,
        belongs,
    )


def release_notes_html(notes_markdown: str, output_file: pathlib.Path) -> None:
    """Generate release notes in HTML format, typically for local testing."""
    md = markdown.Markdown(
        extensions=["pymdownx.tilde", "pymdownx.details"],
    )

    with open(output_file, "w") as output:
        output.write(md.convert(notes_markdown))
        subprocess.Popen(["open", output.name])


def release_notes_markdown(
    ic_repo: GitRepo,
    base_release_tag: str,
    base_release_commit: str,
    release_tag: str,
    release_commit: str,
    os_kind: OsKind,
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

Please note that some commits may be excluded from this release if they're not relevant, or not modifying the {os_kind} image.
Additionally, descriptions of some changes might have been slightly modified to fit the release notes format.

To see a full list of commits added since last release, compare the revisions on [GitHub](https://github.com/dfinity/ic/compare/{base_release_tag}...{release_tag}).
""".format(
        base_release_tag=base_release_tag,
        base_release_commit=base_release_commit,
        release_tag=release_tag,
        release_commit=release_commit,
        reviewers_text=reviewers_text,
        os_kind=os_kind,
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
        if change["exclusion_reason"] or not change["belongs_to_this_release"]:
            text = "~~{} [AUTO-EXCLUDED:{}]~~".format(
                text,
                f"Not modifying {os_kind}"
                if not change["belongs_to_this_release"]
                else change["exclusion_reason"],
            )
        return "* " + text + "\n"

    non_belonging_change = []
    for current_type in sorted(TYPE_PRETTY_MAP, key=lambda x: TYPE_PRETTY_MAP[x][1]):
        if current_type not in change_infos or not change_infos[current_type]:
            continue
        notes += "## {0}:\n".format(TYPE_PRETTY_MAP[current_type][0])

        for change in sorted(
            change_infos[current_type], key=lambda x: ",".join(x["teams"])
        ):
            if not change["belongs_to_this_release"]:
                non_belonging_change.append(change)
                continue
            notes += format_change(change)

    if non_belonging_change:
        notes += f"## ~~Other changes not modifying {os_kind}~~\n"
        for change in non_belonging_change:
            notes += format_change(change)
    return notes


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate release notes")
    parser.add_argument("base_release_tag", type=str, help="base release tag")
    parser.add_argument("base_release_commit", type=str, help="base release commit")
    parser.add_argument("release_tag", type=str, help="release tag")
    parser.add_argument("release_commit", type=str, help="release commit")
    parser.add_argument(
        "--max-commits",
        type=int,
        default=os.environ.get("MAX_COMMITS", 1000),
        help="Maximum number of commits to include in the release notes.",
    )
    parser.add_argument(
        "--commit-annotator-url",
        type=str,
        dest="commit_annotator_url",
        default="http://localhost:9469/",
        help="Base URL of a commit annotator to use in order to determine commit"
        " relevance for a target when composing release notes; if the value 'local'"
        " is specified, it retrieves annotations using an embedded client that"
        " consults a local Git repository clone of the IC; if 'recreate' is specified"
        " as a value, it ignores any existing annotations and runs a process that"
        " re-annotates every commit involved in the release notes-making process"
        " (this is very slow -- roughly a minute per commit to annotate).",
    )
    parser.add_argument(
        "--verbose",
        "--debug",
        action="store_true",
        dest="verbose",
        help="Bump log level.",
    )
    parser.add_argument(
        "--os-kind",
        default=GUESTOS,
        choices=OS_KINDS,
        help="Release artifact for which the notes should be prepared.",
    )
    parser.add_argument(
        "--output",
        type=pathlib.Path,
        help="Generate an HTML file with the output besides printing"
        " it to standard output, and launch your operating system's HTML viewer;"
        " when running via Bazel, please ensure this is an absolute path.",
    )
    args = parser.parse_args()

    conventional_logging(False, args.verbose)

    ic_repo = GitRepo("https://github.com/dfinity/ic.git", main_branch="master")

    if args.commit_annotator_url is None or args.commit_annotator_url == "local":
        annotator: ChangeDeterminatorProtocol = LocalCommitChangeDeterminator(ic_repo)
    elif args.commit_annotator_url == "recreate":
        annotator = RecreatingCommitChangeDeterminator(ic_repo)
    else:
        annotator = CommitAnnotatorClientCommitChangeDeterminator(
            args.commit_annotator_url
        )

    release_notes = prepare_release_notes(
        OrdinaryReleaseNotesRequest(
            args.release_tag,
            args.release_commit,
            args.base_release_tag,
            args.base_release_commit,
            args.os_kind,
        ),
        os_change_determinator=annotator,
        ic_repo=ic_repo,
        max_commits=args.max_commits,
    )
    print(release_notes)
    if args.output:
        release_notes_html(release_notes, args.output)


if __name__ == "__main__":
    main()
