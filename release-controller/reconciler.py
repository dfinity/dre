import json
import logging
import os
import pathlib
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
from governance import GovernanceCanister
from prometheus import ICPrometheus
import release_index


class ReconcilerState:
    def __init__(self, dir: pathlib.Path):
        self.dir = dir

    def _version_path(self, version: str):
        return self.dir / version

    def version_proposal(self, version: str) -> int | None:
        version_file = self._version_path(version)
        if not version_file.exists():
            return None
        content = open(version_file).read()
        if len(content) == 0:
            return None
        return int(content)

    def proposal_submitted(self, version: str) -> bool:
        return self._version_path(version).exists()

    def mark_submitted(self, version: str):
        self._version_path(version).touch()

    def save_proposal(self, version: str, proposal_id: int):
        if not self.version_proposal(version):
            return
        with open(self._version_path(version), "w") as f:
            f.write(str(proposal_id))


def oldest_active_release(index: release_index.Model, active_versions: list[str]) -> release_index.Release:
    for rc in reversed(index.root.releases):
        for v in rc.versions:
            if v.version in active_versions:
                return rc

    raise RuntimeError("invalid configuration, cannot find an active release")


def versions_to_unelect(
    index: release_index.Model, active_versions: list[str], elected_versions: list[str]
) -> list[str]:
    active_releases_versions = []
    for rc in index.root.releases[: index.root.releases.index(oldest_active_release(index, active_versions)) + 1]:
        for v in rc.versions:
            active_releases_versions.append(v.version)

    return [v for v in elected_versions if v not in active_releases_versions]


class Reconciler:
    def __init__(
        self,
        forum_client: ReleaseCandidateForumClient,
        loader: ReleaseLoader,
        notes_client: ReleaseNotesClient,
        publish_client: PublishNotesClient,
        nns_url: str,
        state: ReconcilerState,
    ):
        self.forum_client = forum_client
        self.loader = loader
        self.notes_client = notes_client
        self.publish_client = publish_client
        self.nns_url = nns_url
        self.governance_canister = GovernanceCanister()
        self.state = state
        self.ic_prometheus = ICPrometheus(url="https://victoria.mainnet.dfinity.network/select/0/prometheus")

    def reconcile(self):
        config = self.loader.index()
        active_versions = self.ic_prometheus.active_versions()
        blessed_versions = json.loads(IcAdmin(self.nns_url)._ic_admin_run("get-blessed-replica-versions", "--json"))[
            "value"
        ]["blessed_version_ids"]
        # TODO: get active releases only
        for rc in config.root.releases:
            rc_forum_topic = self.forum_client.get_or_create(rc)
            # update to create posts for any releases
            rc_forum_topic.update(changelog=self.loader.changelog, proposal=self.state.version_proposal)
            for v in rc.versions:
                # TODO: push tag. maybe publish later?
                self.notes_client.ensure(v.version, "TODO:")
                if v.release_notes_ready:
                    changelog = self.notes_client.markdown_file(v.version)
                    if not changelog:
                        logging.warn(f"changelog for version {v.version} not found in google docs")
                        continue
                    self.publish_client.ensure_published(version=v.version, changelog=changelog)

                changelog = self.loader.changelog(v.version)
                if changelog:
                    if not self.state.proposal_submitted(v.version):
                        # this is a defensive approach in case the ic-admin run fails but still manages to submit the proposal. we had cases like this in the past
                        self.state.mark_submitted(v.version)

                        unelect_versions_args = []
                        if v.version == rc.versions[0].version:
                            prometheus = ICPrometheus(
                                url="https://victoria.mainnet.dfinity.network/select/0/prometheus"
                            )
                            active_versions = prometheus.active_versions()
                            unelect_versions_args.append("--replica-versions-to-unelect")
                            unelect_versions_args.extend(
                                versions_to_unelect(
                                    config,
                                    active_versions=active_versions,
                                    elected_versions=json.loads(
                                        IcAdmin(self.nns_url)._ic_admin_run("get-blessed-replica-versions", "--json")
                                    )["value"]["blessed_version_ids"],
                                ),
                            )

                        summary = changelog  # TODO: add forum link to the proposal
                        # TODO: add unelect if first release
                        # TODO: add version to propose
                        # TODO: verify that artifacts exist and checksum match
                        IcAdmin(self.nns_url)._ic_admin_run(
                            "propose-to-update-elected-replica-versions",
                            "--proposal-title",
                            f"Elect new IC/Replica revision (commit {v.version[:7]})",
                            "--summary",
                            summary,
                            "--dry-run",  # TODO: remove
                            "--release-package-sha256-hex",
                            # TODO: hex
                            "--release-package-urls",
                            # TODO: urls
                            "--replica-version-to-elect",
                            v.version,
                            *unelect_versions_args,
                        )

                    versions_proposals = self.governance_canister.replica_version_proposals()
                    if v.version in versions_proposals:
                        self.state.save_proposal(v.version, versions_proposals[v.version])

            # update the forum posts in case the proposal was created
            rc_forum_topic.update(changelog=self.loader.changelog, proposal=self.state.version_proposal)


def main():
    load_dotenv()

    discourse_client = DiscourseClient(
        host=os.environ["DISCOURSE_URL"],
        api_username=os.environ["DISCOURSE_USER"],
        api_key=os.environ["DISCOURSE_KEY"],
    )
    config_loader = GitReleaseLoader() if "dev" not in os.environ else DevReleaseLoader()
    state = ReconcilerState(pathlib.Path.home() / ".cache/release-controller")
    forum_client = ReleaseCandidateForumClient(
        discourse_client,
    )
    github_client = Github(auth=Auth.Token(os.environ["GITHUB_TOKEN"]))
    reconciler = Reconciler(
        forum_client=forum_client,
        loader=config_loader,
        notes_client=ReleaseNotesClient(credentials_file=pathlib.Path(__file__).parent.resolve() / "credentials.json"),
        publish_client=PublishNotesClient(github_client.get_repo("dfinity/dre")),
        nns_url="TODO:",
        state=state,
    )

    # TODO: loop
    reconciler.reconcile()
    # while True:
    #     time.sleep(10)
    # TODO: only check active RCs and newer. e.g., if mainnet has version B & C currently dpeloyed to subnet, and version D on the way, we don't need to do anything about version A
    # TODO: skip initial releases already managed in old way


if __name__ == "__main__":
    main()
