import logging
import os
import re
import typing

from dotenv import load_dotenv
from github import Auth
from github import Github
from github.Repository import Repository
from itertools import groupby
from google_docs import ReleaseNotesClient
from release_notes import PreparedReleaseNotes
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

    def ensure_published(self, version: str, changelog: str, os_kind: OsKind) -> None:
        """Publish the release notes for the given version."""
        logger = LOGGER.getChild(version)
        reldir = release_directory(os_kind)
        published_releases = self.repo.get_contents(f"/{reldir}")
        if not isinstance(published_releases, list):
            return
        if any(version in f.path for f in published_releases):
            return

        branch_name = f"replica-release-notes-{version}"
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

        try:
            logger.info("Creating file on branch %s", branch_name)
            self.repo.create_file(
                path=version_path,
                message=f"chore(release): Elect version {version}",
                content=changelog,
                branch=branch_name,
            )
        except:  # pylint: disable=bare-except  # noqa: E722
            logger.warning("Failed to create file on branch %s", branch_name)

        logger.info(
            "Creating pull request for branch %s — please approve the PR at your leisure",
            branch_name,
        )
        self.repo.create_pull(
            title=f"chore(release): Elect version {version}",
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
                "Could not find release notes section for version %s" % version
            )

        if not re.match(
            r"^Review checklist=+Please cross(\\|)-out your team once you finished the review\s*$",
            changelog[:release_notes_start].replace("\n", ""),
        ):
            logger.info("Release notes not yet ready")
            return

        changelog = changelog[release_notes_start:]
        if check_number_of_changes(changelog) == 0:
            raise ValueError(
                "Release notes for version %s contain no commits that would be published"
                % version
            )
            return
        # TODO: parse markdown to check formatting is correct
        self.ensure_published(version=version, changelog=changelog, os_kind=os_kind)


def check_number_of_changes(changelog: str) -> int:
    BEGINNING_MARKER = "To see a full list of commits added since last release"
    ENDING_MARKER = "## Excluded Changes"

    num_changes = 0
    found_beginning = False
    for line in changelog.splitlines():
        LOGGER.debug("Processing line whole: %s", line)
        if not found_beginning and line.startswith(BEGINNING_MARKER):
            found_beginning = True
            continue

        if found_beginning:
            LOGGER.debug("Processing line: %s", line)
            if line.startswith(ENDING_MARKER):
                break
            if line.startswith("*"):
                num_changes += 1

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
