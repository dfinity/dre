import logging
import os
import re
import typing

from dotenv import load_dotenv
from github import Auth
from github import Github
from github import UnknownObjectException
from github.ContentFile import ContentFile
from github.Repository import Repository
from itertools import groupby
from google_docs import ReleaseNotesClient
from release_notes_composer import PreparedReleaseNotes
import pathlib

from const import OsKind, GUESTOS

REPLICA_RELEASES_DIR = "replica-releases"
HOSTOS_RELEASES_DIR = "hostos-releases"
LOGGER = logging.getLogger(__name__)


def release_directory(os_kind: OsKind) -> str:
    return REPLICA_RELEASES_DIR if os_kind == GUESTOS else HOSTOS_RELEASES_DIR


def post_process_release_notes(release_notes: str) -> str:
    """Process the release notes."""
    lines = [
        # add ticks around commit hash
        re.sub(
            r"(?<=\[)(~*[a-h0-9]{9}~*)(?=\])",
            r"`\g<1>`",
            # remove author
            re.sub(r"(?<=^\* )(.*)author:[^|]+\| ?", r"\g<1>", line),
        )
        for line in release_notes.split("\n")
    ]

    changelog = "\n".join([line for line in lines if "~~" not in line])
    excluded_lines = [line for line in lines if "~~" in line]
    excluded_changes = [
        ln
        for ln in [
            re.sub(
                # remove whitespace after *
                r"(?<=^\* )\s+",
                "",
                # remove ~~
                line.replace("~~", ""),
            ).strip()
            for line in excluded_lines
        ]
        if ln.startswith("* [")
    ]

    EXCLUSION_REGEX = r"\\*\[AUTO\\*-EXCLUDED:([^]]+)\]"

    def exclusion_reason(line: str) -> str:
        m = re.search(EXCLUSION_REGEX, line)
        if not m:
            return "Excluded by authors"
        return m.group(1)

    if excluded_changes:
        changelog += "\n\n## Excluded Changes\n"
        for the_reason, these_excluded_lines in groupby(
            sorted(excluded_changes, key=exclusion_reason), exclusion_reason
        ):
            changelog += (
                f"\n### {the_reason}\n"
                + "\n".join(
                    [
                        re.sub(EXCLUSION_REGEX, "", line).strip()
                        for line in these_excluded_lines
                    ]
                )
                + "\n"
            )

    # remove empty sections
    changelog = re.sub(r"[^\n]+\n-+\n(?!\s*\*)", "", changelog, flags=re.S)
    changelog = re.sub(r"\n{3,}", "\n\n", changelog, flags=re.S)
    return changelog


class PublishNotesClientProtocol(typing.Protocol):
    def publish_if_ready(
        self,
        google_doc_markdownified: PreparedReleaseNotes | None,
        version: str,
        os_kind: OsKind,
    ) -> None: ...


class PublishNotesClient:
    """Publishes release notes on slack."""

    def __init__(self, repo: Repository):
        """Initialize the client with the given repository."""
        self.repo = repo

    def _write_changelog(
        self, version_path: str, changelog: str, branch_name: str, msg: str
    ) -> None:
        """
        Create the changelog on the release notes branch, or update it in place
        if it is already there but stale.

        ``Repository.create_file`` is the GitHub *create* endpoint and fails with
        422 when the path already exists, so a branch left over from an earlier
        reconciler pass keeps its original changelog forever.  That is not
        hypothetical: notes generated before a ``changelog_base`` override landed
        in ``release-index.yaml`` stayed on the branch while the Google Doc was
        regenerated against the new base, so the resulting pull request and the
        doc disagreed about which commits the release contained.

        Writing is skipped entirely when the committed content already matches,
        so a reconciler that runs every 30 seconds does not push an identical
        commit on every pass.
        """
        logger = LOGGER.getChild(branch_name)
        try:
            existing: ContentFile | list[ContentFile] | None = self.repo.get_contents(
                version_path, ref=branch_name
            )
        except UnknownObjectException:
            existing = None

        if existing is None:
            logger.info("Creating %s on branch %s", version_path, branch_name)
            self.repo.create_file(
                path=version_path,
                message=msg,
                content=changelog,
                branch=branch_name,
            )
            return

        if isinstance(existing, list):
            raise RuntimeError(
                f"Expected {version_path} on branch {branch_name} to be a file,"
                " but the GitHub API returned a directory listing."
            )

        if existing.decoded_content.decode("utf-8") == changelog:
            logger.debug(
                "%s on branch %s already matches the changelog; nothing to commit.",
                version_path,
                branch_name,
            )
            return

        logger.info(
            "Updating stale %s on branch %s (the committed changelog differs from"
            " the one just prepared).",
            version_path,
            branch_name,
        )
        self.repo.update_file(
            path=version_path,
            message=msg,
            content=changelog,
            sha=existing.sha,
            branch=branch_name,
        )

    def ensure_published(self, version: str, changelog: str, os_kind: OsKind) -> None:
        """Publish the release notes for the given version."""
        logger = LOGGER.getChild(version)
        reldir = release_directory(os_kind)
        published_releases = self.repo.get_contents(f"/{reldir}")
        if not isinstance(published_releases, list):
            return
        if any(version in f.path for f in published_releases):
            return

        branch_name = (
            f"{'replica' if os_kind == 'GuestOS' else os_kind}-release-notes-{version}"
        )
        pull_head = f"dfinity:{branch_name}"
        if self.repo.get_pulls(head=pull_head, state="open").totalCount > 0:
            logger.info(
                "Waiting for PR of branch %s to be approved and merged", branch_name
            )
            return

        version_path = f"{reldir}/{version}.md"
        if not [b for b in self.repo.get_branches() if b.name == branch_name]:
            logger.info("Creating branch %s", branch_name)
            self.repo.create_git_ref(
                ref=f"refs/heads/{branch_name}",
                sha=self.repo.get_branch("main").commit.sha,
            )

        msg = f"chore(release): Elect version {version} as {os_kind} candidate for rollout"
        try:
            self._write_changelog(
                version_path=version_path,
                changelog=changelog,
                branch_name=branch_name,
                msg=msg,
            )
        except Exception:
            # Deliberately do NOT fall through to create_pull() here.  A pull
            # request opened on top of a failed write advertises a changelog that
            # was never committed -- which is exactly how a branch carrying
            # pre-changelog_base notes ended up in a PR that disagreed with its
            # Google Doc.  Bail out and let the next reconciler pass retry.
            logger.exception(
                "Failed to write %s on branch %s; not opening a pull request"
                " because it would advertise content that was never committed.",
                version_path,
                branch_name,
            )
            return

        logger.info(
            "Creating pull request for branch %s — please approve the PR at your leisure",
            branch_name,
        )
        self.repo.create_pull(
            title=msg,
            base="main",
            head=pull_head,
        )

    def publish_if_ready(
        self,
        google_doc_markdownified: PreparedReleaseNotes | None,
        version: str,
        os_kind: OsKind,
    ) -> None:
        """Publish the release notes if they are ready."""
        logger = LOGGER.getChild(version)
        if not isinstance(google_doc_markdownified, str):
            logger.warning("Didn't get Markdown notes, skipping")
            return

        changelog = post_process_release_notes(google_doc_markdownified)

        release_notes_start = changelog.find("Release Notes")
        if release_notes_start == -1:
            raise ValueError(
                f"{os_kind}: Could not find release notes section for version {version}"
            )

        # Attempt to find NO text between the Review checklist sentence and the Release notes headline.
        # The post_process_release_notes function above should have removed all crossed-out teams
        # from the list of teams that are supposed to review the changelog.  If that list was empty
        # because all teams elected to not review the changelog, then this should immediately succeed
        # and the reconciler (which calls this code) should proceed immediately with publishing the
        # post-processed changelog (from Google Drive) to Github.
        intro_raw = changelog[:release_notes_start]
        # Ready only if there are no remaining non-crossed reviewer bullets
        remaining_bullets = [
            ln for ln in intro_raw.splitlines() if re.match(r"^\s*[-*]\s", ln)
        ]
        if remaining_bullets:
            logger.info(
                "%s: Release notes not yet ready for version %s", os_kind, version
            )
            logger.info("Intro section lines before 'Release Notes for':")
            for idx, line in enumerate(intro_raw.splitlines(), start=1):
                logger.info("  %02d| %s", idx, line)
            return

        changelog = changelog[release_notes_start:]
        if check_number_of_changes(changelog) == 0:
            raise ValueError(
                "Release notes for version %s contain no commits that would be published"
                % version
            )
        # TODO: parse markdown to check formatting is correct
        self.ensure_published(version=version, changelog=changelog, os_kind=os_kind)


def check_number_of_changes(changelog: str) -> int:
    BEGINNING_MARKER = "To see a full list of commits added since last release"
    ENDING_MARKER = "## Excluded Changes"

    num_changes = 0
    found_beginning = False
    for line in changelog.splitlines():
        if not found_beginning and line.startswith(BEGINNING_MARKER):
            found_beginning = True
            continue

        if found_beginning:
            if line.startswith(ENDING_MARKER):
                break
            if line.startswith("*"):
                num_changes += 1

    LOGGER.debug("Found %s changes", num_changes)
    return num_changes


def main() -> None:
    load_dotenv()
    github_client = Github(auth=Auth.Token(os.environ["GITHUB_TOKEN"]))
    client = PublishNotesClient(github_client.get_repo("dfinity/dre-testing"))
    client.ensure_published("85bd56a70e55b2cea75cae6405ae11243e5fdad8", "test", GUESTOS)

    # For testing the `check_number_of_changes`
    release_notes_client = ReleaseNotesClient(
        credentials_file=pathlib.Path(
            os.environ.get(
                "GDOCS_CREDENTIALS_PATH",
                pathlib.Path(__file__).parent.resolve() / "credentials.json",
            )
        )
    )
    # Would not publish this one
    version = "c6847128f3a872e0e084b2920bfcd21f881c69fa"
    # Should publish this one
    # version = "f88938214b16584075196e13d0af7c50f671131a"
    client.publish_if_ready(
        release_notes_client.markdown_file(version, GUESTOS), version, GUESTOS
    )


if __name__ == "__main__":
    main()
