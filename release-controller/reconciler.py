import hashlib
import json
import logging
import os
import pathlib
import sys
import time
import traceback

import requests
import __fix_import_paths
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
import urllib.request
import tempfile
from release_notes import release_notes
from util import version_name
from git_repo import GitRepo, push_release_tags


class ReconcilerState:
    def __init__(self, dir: pathlib.Path):
        if not dir.exists():
            os.makedirs(dir)
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
        if self.version_proposal(version) or not self._version_path(version).exists():
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


# https://stackoverflow.com/a/44873382
def sha256sum(filename):
    h = hashlib.sha256()
    b = bytearray(128 * 1024)
    mv = memoryview(b)
    with open(filename, "rb", buffering=0) as f:
        for n in iter(lambda: f.readinto(mv), 0):
            h.update(mv[:n])
    return h.hexdigest()


def version_package_urls(version: str):
    return [
        f"https://download.dfinity.systems/ic/{version}/guest-os/update-img/update-img.tar.gz",
        f"https://download.dfinity.network/ic/{version}/guest-os/update-img/update-img.tar.gz",
    ]


def version_package_checksum(version: str):
    with tempfile.TemporaryDirectory() as d:
        response = requests.get(f"https://download.dfinity.systems/ic/{version}/guest-os/update-img/SHA256SUMS")
        checksum = [l for l in response.content.decode('utf-8').splitlines() if l.strip().endswith("update-img.tar.gz")][0].split(" ")[
            0
        ]

        for i, u in enumerate(version_package_urls(version)):
            image_file = str(pathlib.Path(d) / f"update-img-{i}.tar.gz")
            logging.debug(f"fetching package {u}")
            with open(image_file, "wb") as file:
                response = requests.get(u)
                file.write(response.content)
            if sha256sum(image_file) != checksum:
                raise RuntimeError("checksums do not match")

        return checksum


class Reconciler:
    def __init__(
        self,
        forum_client: ReleaseCandidateForumClient,
        loader: ReleaseLoader,
        notes_client: ReleaseNotesClient,
        publish_client: PublishNotesClient,
        nns_url: str,
        state: ReconcilerState,
        ic_repo: GitRepo,
        ignore_releases=[],
    ):
        self.forum_client = forum_client
        self.loader = loader
        self.notes_client = notes_client
        self.publish_client = publish_client
        self.nns_url = nns_url
        self.governance_canister = GovernanceCanister()
        self.state = state
        self.ic_prometheus = ICPrometheus(url="https://victoria.mainnet.dfinity.network/select/0/prometheus")
        self.ic_repo = ic_repo
        self.ignore_releases = ignore_releases

    def reconcile(self):
        config = self.loader.index()
        active_versions = self.ic_prometheus.active_versions()
        ic_admin = IcAdmin(self.nns_url, git_revision=os.environ.get("IC_ADMIN_VERSION"))
        for rc_idx, rc in enumerate(
            config.root.releases[: config.root.releases.index(oldest_active_release(config, active_versions)) + 1]
        ):
            if rc.rc_name in self.ignore_releases:
                continue
            rc_forum_topic = self.forum_client.get_or_create(rc)
            # update to create posts for any releases
            rc_forum_topic.update(changelog=self.loader.changelog, proposal=self.state.version_proposal)
            for v_idx, v in enumerate(rc.versions):
                push_release_tags(self.ic_repo, rc)
                self.notes_client.ensure(
                    version=v.version,
                    version_name=version_name(rc_name=rc.rc_name, name=v.name),
                    # TODO: might be good to run this inside the notes_client so that it's not called every loop
                    content=release_notes(
                        first_commit=(
                            config.root.releases[rc_idx + 1].versions[0].version  # take first version in previous rc
                            if v_idx == 0
                            else rc.versions[v_idx - 1].version  # take previous version from same rc
                        ),
                        last_commit=v.version,
                        release_name=version_name(rc_name=rc.rc_name, name=v.name),
                    ),
                    tag_teams_on_create=v_idx == 0,
                )

                self.publish_client.publish_if_ready(google_doc_markdownified=self.notes_client.markdown_file(v.version), version=v.version)

                # returns a result only if changelog is published
                changelog = self.loader.changelog(v.version)
                if changelog:
                    if not self.state.proposal_submitted(v.version):
                        unelect_versions = []
                        if v_idx == 0:
                            unelect_versions.extend(
                                versions_to_unelect(
                                    config,
                                    active_versions=active_versions,
                                    elected_versions=json.loads(
                                        ic_admin._ic_admin_run("get-blessed-replica-versions", "--json")
                                    )["value"]["blessed_version_ids"],
                                ),
                            )
                        # this is a defensive approach in case the ic-admin run fails but still manages to submit the proposal. we had cases like this in the past
                        self.state.mark_submitted(v.version)

                        place_proposal(
                            ic_admin=ic_admin,
                            changelog=changelog,
                            version=v.version,
                            forum_post_url=rc_forum_topic.post_url(v.version),
                            unelect_versions=unelect_versions
                        )

                    versions_proposals = self.governance_canister.replica_version_proposals()
                    if v.version in versions_proposals:
                        self.state.save_proposal(v.version, versions_proposals[v.version])

            # update the forum posts in case the proposal was created
            rc_forum_topic.update(changelog=self.loader.changelog, proposal=self.state.version_proposal)


def place_proposal(ic_admin, changelog, version: str, forum_post_url: str, unelect_versions: list[str], dry_run=False):
    unelect_versions_args = []
    if len(unelect_versions) > 0:
        unelect_versions_args.append("--replica-versions-to-unelect")
        unelect_versions_args.extend(unelect_versions)
    summary = changelog + f"\n\nLink to the forum post: {forum_post_url}"
    logging.info(f"submitting proposal for version {version}")
    ic_admin._ic_admin_run(
        "propose-to-update-elected-replica-versions",
        "--proposal-title",
        f"Elect new IC/Replica revision (commit {version[:7]})",
        "--summary",
        summary,
        *(["--dry-run"] if dry_run else []),
        "--proposer", "39", # TODO: replace with system proposer
        "--release-package-sha256-hex",
        version_package_checksum(version),
        "--release-package-urls",
        *version_package_urls(version),
        "--replica-version-to-elect",
        version,
        *unelect_versions_args,
    )

dre_repo = "dfinity/dre"

def main():
    if len(sys.argv) == 2:
        load_dotenv(sys.argv[1])
    else:
        load_dotenv()

    discourse_client = DiscourseClient(
        host=os.environ["DISCOURSE_URL"],
        api_username=os.environ["DISCOURSE_USER"],
        api_key=os.environ["DISCOURSE_KEY"],
    )
    config_loader = (
        GitReleaseLoader(f"https://github.com/{dre_repo}.git")
        if "dev" not in os.environ
        else DevReleaseLoader()
    )
    state = ReconcilerState(pathlib.Path(os.environ.get('RECONCILER_STATE_DIR', pathlib.Path.home() / ".cache/release-controller")))
    forum_client = ReleaseCandidateForumClient(
        discourse_client,
    )
    github_client = Github(auth=Auth.Token(os.environ["GITHUB_TOKEN"]))
    reconciler = Reconciler(
        forum_client=forum_client,
        loader=config_loader,
        notes_client=ReleaseNotesClient(credentials_file=pathlib.Path(os.environ.get('GDOCS_CREDENTIALS_PATH', pathlib.Path(__file__).parent.resolve() / "credentials.json"))),
        publish_client=PublishNotesClient(github_client.get_repo(dre_repo)),
        nns_url="https://ic0.app",
        state=state,
        ignore_releases=[
            "rc--2024-03-06_23-01",
            "rc--2024-03-20_23-01",
        ],
        ic_repo = GitRepo(f"https://oauth2:{os.environ["GITLAB_TOKEN"]}@gitlab.com/dfinity-lab/public/ic.git", main_branch="master"),
    )

    while True:
        try:
            reconciler.reconcile()
        except Exception as e:
            logging.error(traceback.format_exc())
            logging.error(f"failed to reconcile: {e}")
        time.sleep(60)


def oneoff():
    release_loader = GitReleaseLoader(f"https://github.com/{dre_repo}.git")
    version = "463296c0bc82ad5999b70245e5f125c14ba7d090"
    changelog = release_loader.changelog("463296c0bc82ad5999b70245e5f125c14ba7d090")

    ic_admin = IcAdmin("https://ic0.app", git_revision="e5c6356b5a752a7f5912de133000ae60e0e25aaf")
    place_proposal(
        ic_admin=ic_admin,
        changelog=changelog,
        version=version,
        forum_post_url="https://forum.dfinity.org/t/proposal-to-elect-new-release-rc-2024-03-20-23-01/28746/12",
        unelect_versions=[
            "8d4b6898d878fa3db4028b316b78b469ed29f293",
            "85bd56a70e55b2cea75cae6405ae11243e5fdad8",
            "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f",
            "48da85ee6c03e8c15f3e90b21bf9ccae7b753ee6",
            "a2cf671f832c36c0153d4960148d3e676659a747",
        ]
    )


if __name__ == "__main__":
    logging.basicConfig(stream=sys.stdout, level=logging.INFO)
    main()
