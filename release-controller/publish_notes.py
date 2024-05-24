import logging
import os
import re

from dotenv import load_dotenv
from github import Auth
from github import Github
from github.Repository import Repository

REPLICA_RELEASES_DIR = "replica-releases"


class PublishNotesClient:
    """Publishes release notes on slack."""

    def __init__(self, repo: Repository):
        """Initialize the client with the given repository."""
        self.repo = repo

    def ensure_published(self, version: str, changelog: str):
        """Publish the release notes for the given version."""
        branch_name = f"replica-release-notes-{version}"
        pull_head = f"dfinity:{branch_name}"
        if self.repo.get_pulls(head=pull_head, state="all").totalCount > 0:
            return

        version_path = f"{REPLICA_RELEASES_DIR}/{version}.md"
        if not [b for b in self.repo.get_branches() if b.name == branch_name]:
            logging.info("creating branch %s for version %s", branch_name, version)
            self.repo.create_git_ref(ref=f"refs/heads/{branch_name}", sha=self.repo.get_branch("main").commit.sha)

        try:
            logging.info("creating version %s file on branch %s", version, branch_name)
            self.repo.create_file(
                path=version_path, message=f"Elect version {version}", content=changelog, branch=branch_name
            )
        except:  # pylint: disable=bare-except  # noqa: E722
            logging.warning("failed to create version %s file on branch %s", version, branch_name)

        logging.info("creating pull request for %s, branch %s", version, branch_name)
        self.repo.create_pull(title=f"Elect version {version}", base="main", head=pull_head)

    def publish_if_ready(self, google_doc_markdownified, version: str):
        """Publish the release notes if they are ready."""
        if not isinstance(google_doc_markdownified, str):
            logging.info("didn't get markdown notes for %s, skipping", version)
            return

        changelog = google_doc_markdownified
        changelog = "\n".join(
            [
                # add ticks around commit hash
                re.sub(
                    r"(?<=^\* \[)([a-h0-9]{9})(?=\])",
                    r"`\g<1>`",
                    # remove author
                    re.sub(r"(?<=^\* )author:[^|]+\| ", "", line),
                )
                for line in changelog.split("\n")
                # remove crossed out lines (including reviewer checklist)
                if "~~" not in line
            ]
        )

        release_notes_start = changelog.find("Release Notes")
        if release_notes_start == -1:
            logging.warning("could not find release notes section for version %s", version)
            return

        if not re.match(
            r"^Review checklist=+Please cross-out your team once you finished the review\s*$",
            changelog[:release_notes_start].replace("\n", ""),
        ):
            logging.info("release notes for version %s not yet ready", version)
            return

        changelog = changelog[release_notes_start:]
        # TODO: parse markdown to check formatting is correct
        self.ensure_published(version=version, changelog=changelog)


def main():
    load_dotenv()
    github_client = Github(auth=Auth.Token(os.environ["GITHUB_TOKEN"]))
    client = PublishNotesClient(github_client.get_repo("dfinity/dre-testing"))
    client.ensure_published("85bd56a70e55b2cea75cae6405ae11243e5fdad8", "test")


if __name__ == "__main__":
    main()
