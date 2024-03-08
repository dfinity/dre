import logging
import os
import pathlib
import subprocess
import sys
import time
from pydiscourse import DiscourseClient
from pydantic_yaml import parse_yaml_raw_as
from release_index import Model as ReleaseIndexModel
from forum import ReleaseCandidateForumClient, ReleaseCandidateForumPost
from dotenv import load_dotenv
from pylib.ic_admin import IcAdmin
from release_index_loader import ReleaseLoader, DevReleaseLoader, GitReleaseLoader
from google_docs import ReleaseNotesClient
from github import Github, Auth
from publish_notes import PublishNotesClient


class Reconciler:
    def __init__(
        self,
        forum_client: ReleaseCandidateForumClient,
        loader: ReleaseLoader,
        notes_client: ReleaseNotesClient,
        publish_client: PublishNotesClient,
    ):
        self.forum_client = forum_client
        self.loader = loader
        self.notes_client = notes_client
        self.publish_client = publish_client

    def reconcile(self):
        config = self.loader.index()
        # TODO: get active releases only
        for rc in config.root.releases:
            for v in rc.versions:
                self.notes_client.ensure(v.version, "TODO:")
                if v.release_notes_ready:
                    changelog = self.notes_client.markdown_file(v.version)
                    if not changelog:
                        logging.warn(f"changelog for version {v.version} not found in google docs")
                        continue
                    self.publish_client.ensure_published(version=v.version, changelog=changelog)

                changelog = self.loader.changelog(v.version)
                version_elect_proposal_submitted = False # TODO:
                if changelog and version_elect_proposal_submitted:
                    ic_admin = IcAdmin()
                    ic_admin._ic_admin_run("propose-to-elect")



        # post = self.forum_client.get_or_create("rc--2024-02-21_23-06")
        # post.update(
        #     [
        #         ReleaseCandidateForumPost("first random verasdfsion blablabla345."),
        #         ReleaseCandidateForumPost("second random version blablabla."),
        #     ]
        # )

    def elect(self, version: str):
        IcAdmin()
        # TODO: if first version in RC, retire any versions not present in this RC or previous two RCs


def main():
    load_dotenv()

    discourse_client = DiscourseClient(
        host=os.environ["DISCOURSE_URL"],
        api_username=os.environ["DISCOURSE_USER"],
        api_key=os.environ["DISCOURSE_KEY"],
    )
    forum_client = ReleaseCandidateForumClient(discourse_client)
    github_client = Github(auth=Auth.Token(os.environ["GITHUB_TOKEN"]))
    config_loader = GitReleaseLoader() if "dev" not in os.environ else DevReleaseLoader()
    reconciler = Reconciler(
        forum_client=forum_client,
        loader=config_loader,
        notes_client=ReleaseNotesClient(credentials_file=pathlib.Path(__file__).parent.resolve() / "credentials.json"),
        publish_client=PublishNotesClient(github_client.get_repo("dfinity/dre")),
    )

    reconciler.reconcile()
    # while True:
    #     time.sleep(10)
    # TODO: only check active RCs and newer. e.g., if mainnet has version B & C currently dpeloyed to subnet, and version D on the way, we don't need to do anything about version A


if __name__ == "__main__":
    main()
