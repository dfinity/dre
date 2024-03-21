import os
import logging
from dotenv import load_dotenv
from github.Repository import Repository
from github import Github, Auth
import re

REPLICA_RELEASES_DIR = "replica-releases"


class PublishNotesClient:
    def __init__(self, repo: Repository):
        self.repo = repo

    def ensure_published(self, version: str, changelog: str):
        branch_name = f"replica-release-notes-{version}"
        pull_head = f"LittleChimera:{branch_name}"
        if self.repo.get_pulls(head=pull_head, state="all").totalCount > 0:
            return

        version_path = f"{REPLICA_RELEASES_DIR}/{version}.md"
        if not [b for b in self.repo.get_branches() if b.name == branch_name]:
            logging.info(f"creating branch {branch_name} for version {version}")
            self.repo.create_git_ref(ref=f"refs/heads/{branch_name}", sha=self.repo.get_branch("main").commit.sha)

        try:
            logging.info(f"creating version {version} file on branch {branch_name}")
            self.repo.create_file(
                path=version_path, message=f"Elect version {version}", content=changelog, branch=branch_name
            )
        except:
            logging.warn(f"failed to create version {version} file on branch {branch_name}")

        logging.info(f"creating pull request for {version}, branch {branch_name}")
        self.repo.create_pull(title=f"Elect version {version}", base="main", head=pull_head)

    def publish_if_ready(self, google_doc_markdownified, version: str):
        if not isinstance(google_doc_markdownified, str):
            logging.info(f"didn't get markdown notes for {version}, skipping")
            return

        changelog = google_doc_markdownified
        changelog = "\n".join(
            [
                # add ticks around commit hash
                re.sub(
                    r"(?<=^\* \[)([a-h0-9]{9})(?=\])",
                    r"`\g<1>`",
                    # remove author
                    re.sub(r"(?<=^\* )author:[^|]+\| ", "", l),
                )
                for l in changelog.split("\n")
                # remove crossed out lines (including reviewer checklist)
                if not "~~" in l
            ]
        )

        release_notes_start = changelog.find("Release Notes")
        if release_notes_start == -1:
            logging.warn(f"could not find release notes section for version {version}")
            return

        if "@team" in changelog[:release_notes_start]:
            logging.info(f"release notes for version {version} not yet ready")
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
