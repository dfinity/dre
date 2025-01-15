import datetime
import hashlib
import logging
import os
import pathlib
import socket
import sys
import tempfile
import time
import typing


sys.path.append(os.path.join(os.path.dirname(__file__)))
import __fix_import_paths  # isort:skip  # noqa: F401 # pylint: disable=W0611
import dre_cli
import dryrun
import release_index
import requests
import slack_announce
from dotenv import load_dotenv
from forum import ReleaseCandidateForumClient
from git_repo import GitRepo
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
from release_notes import (
    prepare_release_notes,
    SecurityReleaseNotesRequest,
    OrdinaryReleaseNotesRequest,
)
from util import version_name
from watchdog import Watchdog


class CustomFormatter(logging.Formatter):
    if sys.stderr.isatty():
        green = "\x1b[32;20m"
        yellow = "\x1b[33;20m"
        blue = "\x1b[34;20m"
        red = "\x1b[31;20m"
        bold_red = "\x1b[31;1m"
        reset = "\x1b[0m"
    else:
        green = ""
        yellow = ""
        blue = ""
        red = ""
        bold_red = ""
        reset = ""
    fmt = "%(asctime)s %(levelname)8s  %(name)-37s — %(message)s"
    longfmt = (
        "%(asctime)s %(levelname)8s  %(name)-37s\n"
        "                                  %(message)s"
    )

    FORMATS = {
        logging.DEBUG: green + fmt + reset,
        logging.INFO: blue + fmt + reset,
        logging.WARNING: yellow + fmt + reset,
        logging.ERROR: red + fmt + reset,
        logging.CRITICAL: bold_red + fmt + reset,
    }

    LONG_FORMATS = {
        logging.DEBUG: green + longfmt + reset,
        logging.INFO: blue + longfmt + reset,
        logging.WARNING: yellow + longfmt + reset,
        logging.ERROR: red + longfmt + reset,
        logging.CRITICAL: bold_red + longfmt + reset,
    }

    def format(self, record):
        if len(record.name) > 37 or True:
            log_fmt = self.LONG_FORMATS.get(record.levelno)
        else:
            log_fmt = self.FORMATS.get(record.levelno)
        formatter = logging.Formatter(log_fmt)
        return formatter.format(record)


LOGGER = logging.getLogger()


class ReconcilerState:
    """State for the reconciler. This is used to keep track of the proposals that have been submitted."""

    def __init__(self, path: pathlib.Path):
        """Create a new state object."""
        if not path.exists():
            os.makedirs(path)
        self.path = path
        self._logger = logging.getLogger(self.__class__.__name__)

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
        version_path = self._version_path(version)
        if self._version_path(version).exists():
            proposal_id = self.version_proposal(version)
            if proposal_id:
                self._logger.info(
                    "version %s: proposal %s already submitted", version, proposal_id
                )
            else:
                last_modified = datetime.datetime.fromtimestamp(
                    os.path.getmtime(version_path)
                )
                remaining_time_until_retry = datetime.timedelta(minutes=10) - (
                    datetime.datetime.now() - last_modified
                )
                if remaining_time_until_retry.total_seconds() > 0:
                    self._logger.warning(
                        "version %s: earlier proposal submission attempted but most likely failed, will retry in %s seconds",
                        version,
                        remaining_time_until_retry.total_seconds(),
                    )
                else:
                    os.remove(version_path)
                    return False
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


def oldest_active_release(
    index: release_index.Model, active_versions: list[str]
) -> release_index.Release:
    for rc in reversed(index.root.releases):
        for v in rc.versions:
            if v.version in active_versions:
                return rc

    raise RuntimeError("invalid configuration, cannot find an active release")


def versions_to_unelect(
    index: release_index.Model, active_versions: list[str], elected_versions: list[str]
) -> list[str]:
    active_releases_versions = []
    for rc in index.root.releases[
        : index.root.releases.index(oldest_active_release(index, active_versions)) + 1
    ]:
        for v in rc.versions:
            active_releases_versions.append(v.version)

    return [
        v
        for v in elected_versions
        if v not in active_releases_versions and v not in active_versions
    ]


def find_base_release(
    ic_repo: GitRepo, config: release_index.Model, commit: str
) -> typing.Tuple[str, str]:
    """
    Find the parent release commit for the given commit. Optionally return merge base if it's not a direct parent.
    """
    ic_repo.fetch()
    rc, rc_idx = next(
        (rc, i)
        for i, rc in enumerate(config.root.releases)
        if any(v.version == commit for v in rc.versions)
    )
    v_idx = next(
        i
        for i, v in enumerate(config.root.releases[rc_idx].versions)
        if v.version == commit
    )
    return (
        (
            config.root.releases[rc_idx + 1].versions[0].version,
            version_name(
                config.root.releases[rc_idx + 1].rc_name,
                config.root.releases[rc_idx + 1].versions[0].name,
            ),
        )  # take first version from the previous rc
        if v_idx == 0
        else min(
            [
                (v.version, version_name(rc.rc_name, v.name))
                for v in rc.versions
                if v.version != commit
            ],
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


def version_package_urls(version: str) -> list[str]:
    return [
        f"https://download.dfinity.systems/ic/{version}/guest-os/update-img/update-img.tar.zst",
        f"https://download.dfinity.network/ic/{version}/guest-os/update-img/update-img.tar.zst",
    ]


def version_package_checksum(version: str) -> str:
    with tempfile.TemporaryDirectory() as d:
        response = requests.get(
            f"https://download.dfinity.systems/ic/{version}/guest-os/update-img/SHA256SUMS",
            timeout=10,
        )
        checksum = [
            line
            for line in response.content.decode("utf-8").splitlines()
            if line.strip().endswith("update-img.tar.zst")
        ][0].split(" ")[0]

        for i, u in enumerate(version_package_urls(version)):
            image_file = str(pathlib.Path(d) / f"update-img-{i}.tar.zst")
            LOGGER.getChild("version_package_checksum").debug("fetching package %s", u)
            with open(image_file, "wb") as file:
                response = requests.get(u, timeout=10)
                file.write(response.content)
            if sha256sum(image_file) != checksum:
                raise RuntimeError("checksums do not match")

        return checksum


class ActiveVersionProvider(typing.Protocol):
    def active_versions(self) -> list[str]: ...


class ReplicaVersionProposalProvider(typing.Protocol):
    def replica_version_proposals(self) -> dict[str, int]: ...


class SlackAnnouncerProtocol(typing.Protocol):
    def announce_release(
        self, webhook: str, version_name: str, google_doc_url: str, tag_all_teams: bool
    ) -> None: ...


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
        active_version_provider: ActiveVersionProvider,
        replica_version_proposal_provider: ReplicaVersionProposalProvider,
        dre: dre_cli.DRECli,
        slack_announcer: SlackAnnouncerProtocol,
        ignore_releases=None,
    ):
        """Create a new reconciler."""
        self.forum_client = forum_client
        self.loader = loader
        self.notes_client = notes_client
        self.publish_client = publish_client
        self.nns_url = nns_url
        self.governance_canister = replica_version_proposal_provider
        self.state = state
        self.ic_prometheus = active_version_provider
        self.ic_repo = ic_repo
        self.ignore_releases = ignore_releases or []
        self.dre = dre
        self.slack_announcer = slack_announcer

    def reconcile(self):
        """Reconcile the state of the network with the release index."""
        config = self.loader.index()
        active_versions = self.ic_prometheus.active_versions()
        logger = LOGGER.getChild("reconciler")

        logger.info(
            "GuestOS versions active on subnets or unassigned nodes: %s",
            active_versions,
        )
        dre = self.dre
        for _rc_idx, rc in enumerate(
            config.root.releases[
                : config.root.releases.index(
                    oldest_active_release(config, active_versions)
                )
                + 1
            ]
        ):
            rclogger = logger.getChild(f"{rc.rc_name}")

            if rc.rc_name in self.ignore_releases:
                rclogger.debug("In ignore list.  Skipping.")
                continue

            # update to create posts for any releases
            rclogger.debug("Ensuring forum post for release candidate exists.")
            rc_forum_topic = self.forum_client.get_or_create(rc)

            rclogger.debug("Updating forum posts preemptively.")
            rc_forum_topic.update(
                summary_retriever=self.loader.proposal_summary,
                proposal_id_retriever=self.state.version_proposal,
            )

            for v_idx, v in enumerate(rc.versions):
                release_tag, release_commit, is_security_fix = (
                    version_name(rc_name=rc.rc_name, name=v.name),
                    v.version,
                    v.security_fix,
                )
                revlogger = rclogger.getChild(f"{release_tag}")
                revlogger.debug("Processing this version.")

                if not self.notes_client.has_release_notes(release_commit):
                    revlogger.info("No release notes found.  Creating.")
                    if is_security_fix:
                        revlogger.info(
                            "It's a security fix.  Skipping base release investigation."
                        )
                        # FIXME: how to push the release tags and artifacts
                        # of security fixes 10 days after their rollout?
                        request = SecurityReleaseNotesRequest(
                            release_tag, release_commit
                        )
                    else:
                        revlogger.info(
                            "It's an ordinary release.  Generating full changelog."
                        )
                        self.ic_repo.push_release_tags(rc)
                        base_release_commit, base_release_tag = find_base_release(
                            self.ic_repo, config, release_commit
                        )
                        request = OrdinaryReleaseNotesRequest(
                            release_tag,
                            release_commit,
                            base_release_tag,
                            base_release_commit,
                        )

                    revlogger.info("Preparing release notes.")
                    content = prepare_release_notes(request)

                    revlogger.info("Uploading release notes.")
                    gdoc = self.notes_client.ensure(
                        release_tag=release_tag,
                        release_commit=release_commit,
                        content=content,
                    )

                    if "SLACK_WEBHOOK_URL" in os.environ:
                        # This should have never been in the Google Docs code.
                        revlogger.info("Announcing release notes")
                        self.slack_announcer.announce_release(
                            slack_url=os.environ["SLACK_WEBHOOK_URL"],
                            version_name=release_tag,
                            google_doc_url=gdoc["alternateLink"],
                            tag_all_teams=v_idx == 0,
                        )
                else:
                    revlogger.info("Has release notes.  No need to create them.")

                self.publish_client.publish_if_ready(
                    google_doc_markdownified=self.notes_client.markdown_file(
                        release_commit
                    ),
                    version=release_commit,
                )

                # returns a result only if changelog is published
                changelog = self.loader.proposal_summary(
                    release_commit, is_security_fix
                )
                if not changelog:
                    revlogger.debug("No changelog ready for proposal submission.")
                    continue

                if self.state.proposal_submitted(release_commit):
                    revlogger.debug("Proposal already submitted.")
                else:
                    revlogger.info("Submitting proposal.")
                    unelect_versions = []
                    if v_idx == 0:
                        unelect_versions.extend(
                            versions_to_unelect(
                                config,
                                active_versions=active_versions,
                                elected_versions=dre.get_blessed_versions()["value"][
                                    "blessed_version_ids"
                                ],
                            ),
                        )

                    try:
                        dre.place_proposal(
                            changelog=changelog,
                            version=release_commit,
                            forum_post_url=rc_forum_topic.post_url(release_commit),
                            unelect_versions=unelect_versions,
                            package_checksum=version_package_checksum(release_commit),
                            package_urls=version_package_urls(release_commit),
                        )
                    finally:
                        # This is a defensive approach in case the ic-admin exits with failure
                        # but still manages to submit the proposal, e.g. because it fails to decode the response.
                        # We had cases like this in the past.
                        self.state.mark_submitted(release_commit)

                versions_proposals = (
                    self.governance_canister.replica_version_proposals()
                )
                if release_commit in versions_proposals:
                    self.state.save_proposal(
                        release_commit, versions_proposals[release_commit]
                    )

            rclogger.debug("Updating forum posts after processing versions.")
            # Update the forum posts in case the proposal was created.
            rc_forum_topic.update(
                summary_retriever=self.loader.proposal_summary,
                proposal_id_retriever=self.state.version_proposal,
            )

            rclogger.debug("Finished processing.")

        logger.debug("Finished loop.")


dre_repo = "dfinity/dre"


def main():
    args = sys.argv[1:]
    dry_run = False
    while "--dry-run" in args:
        args.remove("--dry-run")
        dry_run = True

    verbose = False
    for v in ["--verbose", "--debug"]:
        while v in args:
            args.remove(v)
            verbose = True

    root = logging.getLogger()
    root.setLevel(logging.DEBUG if verbose else logging.INFO)
    if verbose:
        for info in ["httpcore", "urllib3", "httpx"]:
            logging.getLogger(info).setLevel(logging.WARNING)

    ch = logging.StreamHandler()
    ch.setLevel(logging.DEBUG if verbose else logging.INFO)
    ch.setFormatter(CustomFormatter())
    root.addHandler(ch)

    # Prep the program for longer timeouts.
    socket.setdefaulttimeout(60)

    if len(args) == 1:
        load_dotenv(args[0])
    else:
        load_dotenv()

    # Watchdog needs to be fed (to report healthy progress) every 10 minutes
    watchdog = Watchdog(timeout_seconds=600)
    watchdog.start()

    discourse_client = (
        DiscourseClient(
            host=os.environ["DISCOURSE_URL"],
            api_username=os.environ["DISCOURSE_USER"],
            api_key=os.environ["DISCOURSE_KEY"],
        )
        if not dry_run
        else dryrun.DiscourseClient()
    )
    config_loader = (
        GitReleaseLoader(f"https://github.com/{dre_repo}.git")
        if "dev" not in os.environ
        else DevReleaseLoader()
    )
    state = ReconcilerState(
        pathlib.Path(
            os.environ.get(
                "RECONCILER_STATE_DIR",
                pathlib.Path.home() / ".cache/release-controller",
            )
        )
    )
    forum_client = (
        ReleaseCandidateForumClient(
            discourse_client,
        )
        if not dry_run
        else dryrun.ForumClient(discourse_client)
    )
    github_client = (
        Github(auth=Auth.Token(os.environ["GITHUB_TOKEN"]))
        if not dry_run
        else dryrun.Github()
    )
    release_notes_client = (
        ReleaseNotesClient(
            credentials_file=pathlib.Path(
                os.environ.get(
                    "GDOCS_CREDENTIALS_PATH",
                    pathlib.Path(__file__).parent.resolve() / "credentials.json",
                )
            )
        )
        if not dry_run
        else dryrun.ReleaseNotesClient()
    )
    ic_repo = (
        GitRepo(
            f"https://oauth2:{os.environ["GITHUB_TOKEN"]}@github.com/dfinity/ic.git",
            main_branch="master",
        )
        if not dry_run
        else dryrun.GitRepo(
            "https://github.com/dfinity/ic.git",
            main_branch="master",
        )
    )
    publish_notes_client = (
        PublishNotesClient(github_client.get_repo(dre_repo))
        if not dry_run
        else dryrun.PublishNotesClient()
    )
    dre = (
        dre_cli.DRECli(
            dre_cli.Auth(
                key_path=os.environ["PROPOSER_KEY_FILE"],
                neuron_id=os.environ["PROPOSER_NEURON_ID"],
            )
        )
        if not dry_run
        else dryrun.DRECli()
    )
    slack_announcer = (
        slack_announce.announce_release if not dry_run else dryrun.MockSlackAnnouncer()
    )

    reconciler = Reconciler(
        forum_client=forum_client,
        loader=config_loader,
        notes_client=release_notes_client,
        publish_client=publish_notes_client,
        nns_url="https://ic0.app",
        state=state,
        ignore_releases=[
            "rc--2024-03-06_23-01",
            "rc--2024-03-20_23-01",
        ],
        ic_repo=ic_repo,
        active_version_provider=ICPrometheus(
            url="https://victoria.mainnet.dfinity.network/select/0/prometheus"
        ),
        replica_version_proposal_provider=GovernanceCanister(),
        slack_announcer=slack_announcer,
        dre=dre,
    )

    while True:
        try:
            reconciler.reconcile()
            watchdog.report_healthy()
            time.sleep(60)
        except Exception as e:
            LOGGER.exception(
                "Failed to reconcile.  Retrying in 15 seconds.  Traceback:"
            )
            time.sleep(60)
        except KeyboardInterrupt:
            LOGGER.info("Interrupted.")
            raise

    LOGGER.info("Exiting.")


# use this as a template in case you need to manually submit a proposal
def oneoff():
    release_loader = GitReleaseLoader(f"https://github.com/{dre_repo}.git")
    version = "ac971e7b4c851b89b312bee812f6de542ed907c5"
    changelog = release_loader.proposal_summary(version)

    dre = dre_cli.DRECli()
    dre.place_proposal(
        changelog=changelog,
        version=version,
        forum_post_url="https://forum.dfinity.org/t/proposal-to-elect-new-release-rc-2024-03-27-23-01/29042/7",
        unelect_versions=[],
        package_checksum=version_package_checksum(version),
        package_urls=version_package_urls(version),
    )


if __name__ == "__main__":
    # FIXME make formatter not output ANSI when stderr is not console
    main()
