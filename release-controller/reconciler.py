import hashlib
import logging
import os
import pathlib
import socket
import sys
import tempfile
import time
import traceback
import typing


sys.path.append(os.path.join(os.path.dirname(__file__)))
import __fix_import_paths  # isort:skip  # noqa: F401 # pylint: disable=W0611
import release_index
import requests
from dotenv import load_dotenv
from forum import ReleaseCandidateForumClient
from git_repo import GitRepo
from git_repo import push_release_tags
from github import Auth
from github import Github
from google_docs import ReleaseNotesClient
from governance import GovernanceCanister
from prometheus import ICPrometheus
from publish_notes import PublishNotesClient
from pydiscourse import DiscourseClient
from release_index_loader import DevReleaseLoader
from release_index_loader import GitReleaseLoader
from release_index_loader import ReleaseLoader
from util import version_name
from watchdog import Watchdog

from pylib.ic_admin import IcAdmin


class ReconcilerState:
    """State for the reconciler. This is used to keep track of the proposals that have been submitted."""

    def __init__(self, path: pathlib.Path):
        """Create a new state object."""
        if not path.exists():
            os.makedirs(path)
        self.path = path

    def _version_path(self, version: str):
        return self.path / version

    def version_proposal(self, version: str) -> int | None:
        """Get the proposal ID for the given version. If the version has not been submitted, return None."""
        version_file = self._version_path(version)
        if not version_file.exists():
            return None
        content = open(version_file, encoding="utf8").read()
        if len(content) == 0:
            return None
        return int(content)

    def proposal_submitted(self, version: str) -> bool:
        """Check if a proposal has been submitted for the given version."""
        if self._version_path(version).exists():
            proposal_id = self.version_proposal(version)
            if proposal_id:
                logging.info("version %s: proposal %s already submitted", version, proposal_id)
            else:
                logging.warning("version %s: earlier proposal submission attempted but failed, will not retry", version)
            return True
        return False

    def mark_submitted(self, version: str):
        """Mark a proposal as submitted."""
        self._version_path(version).touch()

    def save_proposal(self, version: str, proposal_id: int):
        """Save the proposal ID for the given version."""
        if self.version_proposal(version) or not self._version_path(version).exists():
            return
        with open(self._version_path(version), "w", encoding="utf8") as f:
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

    return [v for v in elected_versions if v not in active_releases_versions and v not in active_versions]


def find_base_release(ic_repo: GitRepo, config: release_index.Model, commit: str) -> typing.Tuple[str, str]:
    """
    Find the parent release commit for the given commit. Optionally return merge base if it's not a direct parent.
    """
    ic_repo.fetch()
    rc, rc_idx = next(
        (rc, i) for i, rc in enumerate(config.root.releases) if any(v.version == commit for v in rc.versions)
    )
    v_idx = next(i for i, v in enumerate(config.root.releases[rc_idx].versions) if v.version == commit)
    return (
        (
            config.root.releases[rc_idx + 1].versions[0].version,
            version_name(config.root.releases[rc_idx + 1].rc_name, config.root.releases[rc_idx + 1].versions[0].name),
        )  # take first version from the previous rc
        if v_idx == 0
        else min(
            [(v.version, version_name(rc.rc_name, v.name)) for v in rc.versions if v.version != commit],
            key=lambda v: ic_repo.distance(ic_repo.merge_base(v[0], commit), commit),
        )
    )


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
        response = requests.get(
            f"https://download.dfinity.systems/ic/{version}/guest-os/update-img/SHA256SUMS", timeout=10
        )
        checksum = [
            line for line in response.content.decode("utf-8").splitlines() if line.strip().endswith("update-img.tar.gz")
        ][0].split(" ")[0]

        for i, u in enumerate(version_package_urls(version)):
            image_file = str(pathlib.Path(d) / f"update-img-{i}.tar.gz")
            logging.debug("fetching package %s", u)
            with open(image_file, "wb") as file:
                response = requests.get(u, timeout=10)
                file.write(response.content)
            if sha256sum(image_file) != checksum:
                raise RuntimeError("checksums do not match")

        return checksum


class Reconciler:
    """Reconcile the state of the network with the release index, and create a forum post if needed."""

    def __init__(
        self,
        forum_client: ReleaseCandidateForumClient,
        loader: ReleaseLoader,
        notes_client: ReleaseNotesClient,
        publish_client: PublishNotesClient,
        nns_url: str,
        state: ReconcilerState,
        ic_repo: GitRepo,
        ignore_releases=None,
    ):
        """Create a new reconciler."""
        self.forum_client = forum_client
        self.loader = loader
        self.notes_client = notes_client
        self.publish_client = publish_client
        self.nns_url = nns_url
        self.governance_canister = GovernanceCanister()
        self.state = state
        self.ic_prometheus = ICPrometheus(url="https://victoria.mainnet.dfinity.network/select/0/prometheus")
        self.ic_repo = ic_repo
        self.ignore_releases = ignore_releases or []

    def reconcile(self):
        """Reconcile the state of the network with the release index."""
        config = self.loader.index()
        active_versions = self.ic_prometheus.active_versions()
        logging.info("GuestOS versions active on subnets or unassigned nodes: %s", active_versions)
        ic_admin = IcAdmin(self.nns_url, git_revision=os.environ.get("IC_ADMIN_VERSION"))
        for rc_idx, rc in enumerate(
            config.root.releases[: config.root.releases.index(oldest_active_release(config, active_versions)) + 1]
        ):
            if rc.rc_name in self.ignore_releases:
                continue
            rc_forum_topic = self.forum_client.get_or_create(rc)
            # update to create posts for any releases
            rc_forum_topic.update(changelog=self.loader.proposal_summary, proposal=self.state.version_proposal)
            for v_idx, v in enumerate(rc.versions):
                logging.info("Updating version %s", v)
                push_release_tags(self.ic_repo, rc)
                base_release_commit, base_release_name = find_base_release(self.ic_repo, config, v.version)
                self.notes_client.ensure(
                    base_release_commit=base_release_commit,
                    base_release_tag=base_release_name,
                    release_tag=version_name(rc_name=rc.rc_name, name=v.name),
                    release_commit=v.version,
                    tag_teams_on_create=v_idx == 0,
                )

                self.publish_client.publish_if_ready(
                    google_doc_markdownified=self.notes_client.markdown_file(v.version), version=v.version
                )

                # returns a result only if changelog is published
                changelog = self.loader.proposal_summary(v.version)
                if changelog:
                    if self.state.proposal_submitted(v.version):
                        logging.info("RC %s: proposal already submitted for version %s", rc.rc_name, v.version)
                    else:
                        logging.info("RC %s: submitting proposal for version %s", rc.rc_name, v.version)
                        unelect_versions = []
                        if v_idx == 0:
                            unelect_versions.extend(
                                versions_to_unelect(
                                    config,
                                    active_versions=active_versions,
                                    elected_versions=ic_admin.get_blessed_versions()["value"]["blessed_version_ids"],
                                ),
                            )
                        # This is a defensive approach in case the ic-admin exits with failure
                        # but still manages to submit the proposal, e.g. because it fails to decode the response.
                        # We had cases like this in the past.
                        self.state.mark_submitted(v.version)

                        place_proposal(
                            ic_admin=ic_admin,
                            changelog=changelog,
                            version=v.version,
                            forum_post_url=rc_forum_topic.post_url(v.version),
                            unelect_versions=unelect_versions,
                        )

                    versions_proposals = self.governance_canister.replica_version_proposals()
                    if v.version in versions_proposals:
                        self.state.save_proposal(v.version, versions_proposals[v.version])

            # update the forum posts in case the proposal was created
            rc_forum_topic.update(changelog=self.loader.proposal_summary, proposal=self.state.version_proposal)


def place_proposal(ic_admin, changelog, version: str, forum_post_url: str, unelect_versions: list[str], dry_run=False):
    unelect_versions_args = []
    if len(unelect_versions) > 0:
        unelect_versions_args.append("--replica-versions-to-unelect")
        unelect_versions_args.extend(unelect_versions)
    summary = changelog + f"\n\nLink to the forum post: {forum_post_url}"
    logging.info("submitting proposal for version %s", version)
    ic_admin.ic_admin_run(
        "propose-to-update-elected-replica-versions",
        "--proposal-title",
        f"Elect new IC/Replica revision (commit {version[:7]})",
        "--summary",
        summary,
        *(["--dry-run"] if dry_run else []),
        "--proposer",
        os.environ["PROPOSER_NEURON_ID"],  # TODO: replace with system proposer
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

    watchdog = Watchdog(timeout_seconds=600)  # Reconciler should report healthy every 10 minutes
    watchdog.start()

    discourse_client = DiscourseClient(
        host=os.environ["DISCOURSE_URL"],
        api_username=os.environ["DISCOURSE_USER"],
        api_key=os.environ["DISCOURSE_KEY"],
    )
    config_loader = (
        GitReleaseLoader(f"https://github.com/{dre_repo}.git") if "dev" not in os.environ else DevReleaseLoader()
    )
    state = ReconcilerState(
        pathlib.Path(os.environ.get("RECONCILER_STATE_DIR", pathlib.Path.home() / ".cache/release-controller"))
    )
    forum_client = ReleaseCandidateForumClient(
        discourse_client,
    )
    github_token = os.environ["GITHUB_TOKEN"]
    github_client = Github(auth=Auth.Token(github_token))
    reconciler = Reconciler(
        forum_client=forum_client,
        loader=config_loader,
        notes_client=ReleaseNotesClient(
            credentials_file=pathlib.Path(
                os.environ.get("GDOCS_CREDENTIALS_PATH", pathlib.Path(__file__).parent.resolve() / "credentials.json")
            )
        ),
        publish_client=PublishNotesClient(github_client.get_repo(dre_repo)),
        nns_url="https://ic0.app",
        state=state,
        ignore_releases=[
            "rc--2024-03-06_23-01",
            "rc--2024-03-20_23-01",
        ],
        ic_repo=GitRepo(f"https://oauth2:{github_token}@github.com/dfinity/ic.git", main_branch="master"),
    )

    while True:
        try:
            reconciler.reconcile()
            watchdog.report_healthy()
        except Exception as e:
            logging.error(traceback.format_exc())
            logging.error("failed to reconcile: %s", e)
        time.sleep(60)


# use this as a template in case you need to manually submit a proposal
def oneoff():
    release_loader = GitReleaseLoader(f"https://github.com/{dre_repo}.git")
    version = "ac971e7b4c851b89b312bee812f6de542ed907c5"
    changelog = release_loader.proposal_summary(version)

    ic_admin = IcAdmin("https://ic0.app", git_revision="5ba1412f9175d987661ae3c0d8dbd1ac3e092b7d")
    place_proposal(
        ic_admin=ic_admin,
        changelog=changelog,
        version=version,
        forum_post_url="https://forum.dfinity.org/t/proposal-to-elect-new-release-rc-2024-03-27-23-01/29042/7",
        unelect_versions=[],
    )


if __name__ == "__main__":
    logging.basicConfig(stream=sys.stdout, level=logging.INFO)
    socket.setdefaulttimeout(60)
    main()
