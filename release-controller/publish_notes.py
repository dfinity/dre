import os
import logging
from github.Repository import Repository
from github import Github, Auth

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


def main():
    github_client = Github(auth=Auth.Token(os.environ["GITHUB_TOKEN"]))
    client = PublishNotesClient(github_client.get_repo("dfinity-lab/dre-testing"))
    client.ensure_published("85bd56a70e55b2cea75cae6405ae11243e5fdad8", "test")


if __name__ == "__main__":
    main()
