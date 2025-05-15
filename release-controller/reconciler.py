import argparse
import logging
import os
import pathlib
import requests
import socket
import sys
import time
import typing
import urllib.parse

sys.path.append(os.path.join(os.path.dirname(__file__)))
import dre_cli
import dryrun
import release_index
import slack_announce
import reconciler_state
from const import OsKind, OS_KINDS, GUESTOS, HOSTOS
from dotenv import load_dotenv
from forum import ReleaseCandidateForumClient, ForumClientProtocol
from git_repo import GitRepo
from github import Auth
from github import Github
from google_docs import ReleaseNotesClient, ReleaseNotesClientProtocol
from prometheus import ICPrometheus
from prometheus_client import start_http_server, Gauge
from publish_notes import PublishNotesClient, PublishNotesClientProtocol
from pydiscourse import DiscourseClient
from release_index_loader import DevReleaseLoader
from release_index_loader import GitReleaseLoader
from release_index_loader import ReleaseLoader
from release_notes_composer import (
    prepare_release_notes,
    SecurityReleaseNotesRequest,
    OrdinaryReleaseNotesRequest,
)
from commit_annotation import (
    LocalCommitChangeDeterminator,
    CommitAnnotatorClientCommitChangeDeterminator,
    ChangeDeterminatorProtocol,
    NotReady,
)
from util import version_name, conventional_logging, sha256sum_http_response
from watchdog import Watchdog

# It is safe to delete releases from this list once
# they disappear from file
# https://github.com/dfinity/dre/blob/main/release-index.yaml
IGNORED_RELEASES = [
    "rc--2024-03-06_23-01",
    "rc--2024-03-20_23-01",
    # From here on now we prevent the processing of releases that
    # would screw with the forum posts since their contents and
    # ordering in the threads have changed from this point on,
    # due to the addition of support for HostOS releases.
    "rc--2024-06-26_23-01",
    "rc--2024-07-03_23-01",
    "rc--2024-07-10_23-01",
    "rc--2024-07-25_21-03",
    "rc--2024-08-02_01-30",
    "rc--2024-08-08_07-48",
    "rc--2024-08-15_01-30",
    "rc--2024-08-21_15-36",
    "rc--2024-08-29_01-30",
    "rc--2024-09-06_01-30",
    "rc--2024-09-12_01-30",
    "rc--2024-09-19_01-31",
    "rc--2024-09-26_01-31",
    "rc--2024-10-03_01-30",
    "rc--2024-10-11_14-35",
    "rc--2024-10-17_03-07",
    "rc--2024-10-23_03-07",
    "rc--2024-10-31_03-09",
    "rc--2024-11-07_03-07",
    "rc--2024-11-14_03-07",
    "rc--2024-11-21_03-11",
    "rc--2024-11-28_03-15",
    "rc--2024-12-06_03-16",
    "rc--2025-01-03_03-07",
    "rc--2025-01-09_03-19",
    "rc--2025-01-16_16-18",
    "rc--2025-01-23_03-04",
    "rc--2025-01-30_03-03",
    "rc--2025-02-06_12-26",
    "rc--2025-02-13_03-06",
    "rc--2025-02-20_10-16",
    "rc--2025-02-27_03-09",
    "rc--2025-03-06_03-10",
    "rc--2025-03-14_03-10",
    "rc--2025-03-20_03-11",
    "rc--2025-03-27_03-14",
    "rc--2025-04-03_03-15",
    "rc--2025-04-10_03-16",
    "rc--2025-04-11_13-20",
]

LAST_CYCLE_END_TIMESTAMP_SECONDS = Gauge(
    "last_cycle_end_timestamp_seconds",
    "The UNIX timestamp of the last cycle that completed",
)
LAST_CYCLE_SUCCESS_TIMESTAMP_SECONDS = Gauge(
    "last_cycle_success_timestamp_seconds",
    "The UNIX timestamp of the last cycle that completed successfully",
)
LAST_CYCLE_START_TIMESTAMP_SECONDS = Gauge(
    "last_cycle_start_timestamp_seconds",
    "The UNIX timestamp of the start of the last cycle",
)
LAST_CYCLE_SUCCESSFUL = Gauge(
    "last_cycle_successful",
    "1 if the last cycle was successful, 0 if it was not",
    labelnames=["os_kind", "rc", "version", "phase"],
)

LOGGER = logging.getLogger()


def oldest_active_release(
        index: release_index.Model, active_versions: list[str]
) -> release_index.Release:
    for rc in reversed(index.root.releases):
        for v in rc.versions:
            if v.version in active_versions:
                return rc

    raise ValueError("invalid configuration, cannot find an active release")


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


def version_package_urls(version: str, os_kind: OsKind) -> list[str]:
    v = "host-os" if os_kind == HOSTOS else "guest-os"
    return [
        f"https://download.dfinity.systems/ic/{version}/{v}/update-img/update-img.tar.zst",
        f"https://download.dfinity.network/ic/{version}/{v}/update-img/update-img.tar.zst",
    ]


def version_package_checksum(version: str, os_kind: OsKind) -> str:
    v = "host-os" if os_kind == HOSTOS else "guest-os"
    hashurl = f"https://download.dfinity.systems/ic/{version}/{v}/update-img/SHA256SUMS"
    LOGGER.getChild("version_package_checksum").debug("fetching checksums")
    response = requests.get(hashurl, timeout=10)
    response.raise_for_status()
    checksum = typing.cast(
        str,
        [
            line
            for line in response.content.decode("utf-8").splitlines()
            if line.strip().endswith("update-img.tar.zst")
        ][0].split(" ")[0],
    )

    for u in version_package_urls(version, os_kind):
        LOGGER.getChild("version_package_checksum").debug("fetching package %s", u)
        with requests.get(u, timeout=10, stream=True) as resp:
            resp.raise_for_status()
            actual_sum = sha256sum_http_response(resp, urllib.parse.urlparse(u).netloc)
        if actual_sum != checksum:
            raise ValueError(
                "checksums for %s do not match contents of %s" % (u, hashurl)
            )

    return checksum


class ActiveVersionProvider(typing.Protocol):
    def active_guestos_versions(self) -> list[str]: ...

    def active_hostos_versions(self) -> list[str]: ...


class ReplicaVersionProposalProvider(typing.Protocol):
    def replica_version_proposals(self) -> dict[str, int]: ...

    def hostos_version_proposals(self) -> dict[str, int]: ...


Phase = (
        typing.Literal["forum post creation"]
        | typing.Literal["release notes preparation"]
        | typing.Literal["release notes announcement"]
        | typing.Literal["release notes pull request"]
        | typing.Literal["proposal submission"]
        | typing.Literal["forum post update"]
)


class Failed(object):
    pass


FAILED = Failed()


class VersionState(object):
    """
    Defines the state of completion of processing a release version.

    The following flags represent are the events that must have taken
    place to consider a release processed.
    """

    rc_name: str
    version_name: str
    git_revision: str
    security_fix: bool
    os_kind: OsKind
    is_base: bool
    rc: release_index.Release

    current_phase: Phase | None = None
    has_forum_post: bool | Failed = False
    has_prepared_release_notes: bool | Failed = False
    has_release_notes_announced: bool | Failed = False
    has_release_notes_submitted_as_pr: bool | Failed = False
    has_proposal: bool | Failed = False
    forum_post_updated: bool | Failed = False

    def __init__(
            self,
            rc_name: str,
            version_name: str,
            git_revision: str,
            os_kind: OsKind,
            security_fix: bool,
            is_base: bool,
            rc: release_index.Release,
    ):
        self.rc_name = rc_name
        self.version_name = version_name
        self.git_revision = git_revision
        self.os_kind = os_kind
        self.security_fix = security_fix
        self.is_base = is_base
        self.rc = rc

        self.phase_not_done = False

    @property
    def complete(self) -> bool:
        return self.has_proposal is True and self.forum_post_updated is True

    def __call__(self, phase: Phase) -> "VersionState":
        self.current_phase = phase
        return self

    def __enter__(self) -> "VersionState":
        return self

    def failed(self, phase: Phase) -> None:
        self.current_phase = phase
        self.__exit__(Exception, None, None)

    def incomplete(self) -> None:
        "Mark the phase as not done."
        self.phase_not_done = True

    def completed(self, phase: Phase) -> None:
        "Mark the phase as completed"
        self.current_phase = phase
        self.__exit__(None, None, None)

    def __exit__(
            self,
            exc_type: type[BaseException] | None,
            exc_val: BaseException | None,
            exc_tb: typing.Any,
    ) -> None:
        val = FAILED if exc_type else not (self.phase_not_done)
        if self.current_phase == "forum post creation":
            self.has_forum_post = val
        elif self.current_phase == "release notes preparation":
            self.has_prepared_release_notes = val
        elif self.current_phase == "release notes announcement":
            self.has_release_notes_announced = val
        elif self.current_phase == "release notes pull request":
            self.has_release_notes_submitted_as_pr = val
        elif self.current_phase == "proposal submission":
            self.has_proposal = val
        elif self.current_phase == "forum post update":
            self.forum_post_updated = val
        else:
            assert 0, "phase not reached %s" % self.current_phase
        LAST_CYCLE_SUCCESSFUL.labels(
            self.os_kind,
            self.rc_name,
            self.version_name,
            self.current_phase,
        ).set(0 if exc_type else 1)
        self.current_phase = None
        self.phase_not_done = False


class Reconciler:
    """Reconcile the state of the network with the release index, and create a forum post if needed."""

    def __init__(
            self,
            forum_client: ForumClientProtocol,
            loader: ReleaseLoader,
            notes_client: ReleaseNotesClientProtocol,
            publish_client: PublishNotesClientProtocol,
            nns_url: str,
            state: reconciler_state.ReconcilerState,
            ic_repo: GitRepo,
            change_determinator_factory: typing.Callable[[], ChangeDeterminatorProtocol],
            active_version_provider: ActiveVersionProvider,
            dre: dre_cli.DRECli,
            slack_announcer: slack_announce.SlackAnnouncerProtocol,
            ignore_releases: list[str] | None = None,
    ):
        """Create a new reconciler."""
        self.forum_client = forum_client
        self.loader = loader
        self.notes_client = notes_client
        self.publish_client = publish_client
        self.nns_url = nns_url
        self.state = state
        self.ic_prometheus = active_version_provider
        self.ic_repo = ic_repo
        self.ignore_releases = ignore_releases or []
        self.dre = dre
        self.slack_announcer = slack_announcer
        self.change_determinator_factory = change_determinator_factory
        self.local_release_state: dict[str, dict[str, dict[OsKind, VersionState]]] = {}

    def reconcile(self) -> None:
        """Reconcile the state of the network with the release index."""
        config = self.loader.index()
        active_guestos_versions = self.ic_prometheus.active_guestos_versions()
        active_hostos_versions = self.ic_prometheus.active_hostos_versions()
        logger = LOGGER.getChild("reconciler")

        oldest_active_guestos = oldest_active_release(config, active_guestos_versions)
        oldest_active_hostos = oldest_active_release(config, active_hostos_versions)
        oldest_active_index = max(
            [
                config.root.releases.index(oldest_active_guestos),
                config.root.releases.index(oldest_active_hostos),
            ]
        )
        releases = config.root.releases[: oldest_active_index + 1]
        # Remove ignored releases from list to process.
        releases = [r for r in releases if r.rc_name not in self.ignore_releases]

        # Preload the cache of known successfully processed releases.
        # We will use this information as an operation plan.
        for relcand in releases:
            if relcand.rc_name not in self.local_release_state:
                self.local_release_state[relcand.rc_name] = {}
            for v_idx, rcver in enumerate(relcand.versions):
                if rcver.name not in self.local_release_state[relcand.rc_name]:
                    self.local_release_state[relcand.rc_name][rcver.name] = {}
                for os_kind in OS_KINDS:
                    if os_kind == GUESTOS or v_idx == 0:  # Do HostOS for base release.
                        if (
                                os_kind
                                not in self.local_release_state[relcand.rc_name][rcver.name]
                        ):
                            self.local_release_state[relcand.rc_name][rcver.name][
                                os_kind
                            ] = VersionState(
                                relcand.rc_name,
                                rcver.name,
                                rcver.version,
                                os_kind,
                                rcver.security_fix,
                                is_base=v_idx == 0,
                                rc=relcand,
                            )
                        version = self.local_release_state[relcand.rc_name][rcver.name][
                            os_kind
                        ]
                        # Update this just in case the commit ID changed for this version name.
                        version.git_revision = rcver.version
                        version.security_fix = rcver.security_fix
                        version.is_base = v_idx == 0
                        version.rc = relcand
                        if isinstance(
                                self.state.version_proposal(rcver.version, os_kind),
                                reconciler_state.SubmittedProposal,
                        ):
                            version.completed("proposal submission")

        # Filter the releases to process by removing those which are complete.
        versions = [
            version
            for rc in self.local_release_state.values()
            for version_batch in rc.values()
            for version in version_batch.values()
            if not version.complete
        ]

        if [x for x in versions if x.os_kind == GUESTOS]:
            logger.info(
                "GuestOS versions active on subnets or unassigned nodes: %s",
                ", ".join(active_guestos_versions),
            )
            logger.info(
                "Oldest active GuestOS release: %s", oldest_active_guestos.rc_name
            )
        if [x for x in versions if x.os_kind == HOSTOS]:
            logger.info(
                "HostOS versions active on subnets or unassigned nodes: %s",
                ", ".join(active_hostos_versions),
            )
            logger.info(
                "Oldest active HostOS release: %s", oldest_active_hostos.rc_name
            )

        if versions:
            existing_guestos_proposals, existing_hostos_proposals = (
                self.dre.get_election_proposals_by_version()
            )
        else:
            existing_guestos_proposals, existing_hostos_proposals = ({}, {})

        if versions:
            logger.info("Processing the following release versions:")
            for idx, vv in enumerate(versions):
                logger.info(
                    "%s. %s-%s (%s)", idx + 1, vv.rc_name, vv.version_name, vv.os_kind
                )

        for v in versions:
            rclogger = logger.getChild(f"{v.rc_name}")
            revlogger = rclogger.getChild(f"{v.version_name}.{v.os_kind}")

            phase = v

            release_tag, release_commit, is_security_fix = (
                version_name(rc_name=v.rc_name, name=v.version_name),
                v.git_revision,
                v.security_fix,
            )

            prop = self.state.version_proposal(release_commit, v.os_kind)
            if isinstance(prop, reconciler_state.SubmittedProposal):
                revlogger.debug("%s.  Proposal not needed.", prop)
                phase.completed("proposal submission")
            elif (
                    isinstance(prop, reconciler_state.DREMalfunction)
                    and not prop.ready_to_retry()
            ):
                phase.failed("proposal submission")
                revlogger.debug("%s.  Not ready to retry yet.")
                continue

            # update to create posts for any releases
            with phase("forum post creation"):
                rclogger.debug("Ensuring forum post for release candidate exists.")
                rc_forum_topic = self.forum_client.get_or_create(v.rc)

            needs_announce = False

            with phase("release notes preparation"):
                if markdown_file := self.notes_client.markdown_file(
                        release_commit, v.os_kind
                ):
                    revlogger.info("Has release notes in editor.  Going to next phase.")
                else:
                    revlogger.info("No release notes found in editor.  Creating.")
                    if is_security_fix:
                        revlogger.info(
                            "It's a security fix.  Skipping base release investigation."
                        )
                        # FIXME: how to push the release tags and artifacts
                        # of security fixes 10 days after their rollout?
                        request: (
                                OrdinaryReleaseNotesRequest | SecurityReleaseNotesRequest
                        ) = SecurityReleaseNotesRequest(
                            release_tag, release_commit, v.os_kind
                        )
                    else:
                        revlogger.info(
                            "It's an ordinary release.  Generating full changelog."
                        )
                        self.ic_repo.push_release_tags(v.rc)
                        self.ic_repo.fetch()
                        base_release_commit, base_release_tag = find_base_release(
                            self.ic_repo, config, release_commit
                        )
                        request = OrdinaryReleaseNotesRequest(
                            release_tag,
                            release_commit,
                            base_release_tag,
                            base_release_commit,
                            v.os_kind,
                        )

                    revlogger.info("Preparing release notes.")
                    # FIXME!  Make this pluggable from main().
                    # Big problem is that the change determinator needs
                    # to fetch notes, these are not fetched automatically,
                    # so the client needs to provide an interface to do
                    # this.
                    try:
                        content = prepare_release_notes(
                            request,
                            self.ic_repo,
                            self.change_determinator_factory(),
                        )
                    except NotReady:
                        phase.incomplete()
                        revlogger.warning(
                            "Release notes cannot be prepared because the commit"
                            " annotator is not done annotating all the commits of"
                            " this release.  Verify that the commit annotator is"
                            " operating properly."
                        )
                        continue

                    revlogger.info("Uploading release notes.")
                    gdoc, needs_announce = self.notes_client.ensure(
                        release_tag=release_tag,
                        release_commit=release_commit,
                        content=content,
                        os_kind=v.os_kind,
                    )

            if "SLACK_WEBHOOK_URL" in os.environ and needs_announce:
                with phase("release notes announcement"):
                    # This should have never been in the Google Docs code.
                    revlogger.info("Announcing release notes")
                    self.slack_announcer.announce_release(
                        webhook=os.environ["SLACK_WEBHOOK_URL"],
                        version_name=release_tag,
                        google_doc_url=gdoc["alternateLink"],
                        os_kind=v.os_kind,
                    )

            with phase("release notes pull request") as p:
                self.publish_client.publish_if_ready(
                    google_doc_markdownified=markdown_file,
                    version=release_commit,
                    os_kind=v.os_kind,
                )
                # returns a result only if changelog is published
                changelog = self.loader.proposal_summary(
                    release_commit, v.os_kind, is_security_fix
                )
                if not changelog:
                    revlogger.debug("No changelog ready for proposal submission.")
                    p.incomplete()
                    continue
                else:
                    revlogger.info(
                        "The changelog is now ready for proposal submission."
                    )

            with phase("proposal submission"):
                if isinstance(prop, reconciler_state.SubmittedProposal):
                    revlogger.info(
                        "%s.  We will check if forum post needs update.",
                        prop,
                    )
                elif discovered_proposal := (
                        existing_hostos_proposals
                        if v.os_kind == HOSTOS
                        else existing_guestos_proposals
                ).get(release_commit):
                    if isinstance(prop, reconciler_state.NoProposal):
                        revlogger.warning(
                            "We have no record of a proposal being submitted.  However, contrary"
                            " to our record, proposal to elect %s was indeed successfully"
                            " submitted by someone else as ID %s.",
                            release_commit,
                            discovered_proposal["id"],
                        )
                    elif isinstance(prop, reconciler_state.DREMalfunction):
                        revlogger.warning(
                            "%s.  However, contrary to recorded failure, proposal"
                            " to elect %s was indeed successfully submitted by us as ID %s.",
                            prop,
                            release_commit,
                            discovered_proposal["id"],
                        )
                    prop.record_submission(discovered_proposal["id"])
                else:
                    # No discovered proposal and either prior malfunction or no proposal.
                    # Time to create a proposal proposal.
                    revlogger.info("Preparing proposal for %s", release_commit)
                    try:
                        checksum = version_package_checksum(release_commit, v.os_kind)
                    except requests.exceptions.HTTPError as e:
                        if e.response.status_code == 404:
                            phase.incomplete()
                            revlogger.warning(
                                "Proposal cannot be placed because one of the URLs"
                                " to be fetched does not exist (%s)."
                                "  Verify that the IC OS merge pipeline has uploaded"
                                " all the URLs required for the proposal."
                            )
                            continue

                    urls = version_package_urls(release_commit, v.os_kind)
                    unelect_versions = []
                    if v.is_base == 0:
                        unelect_versions.extend(
                            versions_to_unelect(
                                config,
                                active_versions=(
                                    active_hostos_versions
                                    if v.os_kind == HOSTOS
                                    else active_guestos_versions
                                ),
                                elected_versions=[
                                    v
                                    for v in (
                                        self.dre.get_blessed_hostos_versions()
                                        if v.os_kind == HOSTOS
                                        else self.dre.get_blessed_guestos_versions()
                                    )
                                ],
                            ),
                        )
                    try:
                        proposal_id = self.dre.propose_to_revise_elected_os_versions(
                            changelog=changelog,
                            version=release_commit,
                            os_kind=v.os_kind,
                            forum_post_url=rc_forum_topic.post_url(release_commit),
                            unelect_versions=unelect_versions,
                            package_checksum=checksum,
                            package_urls=urls,
                        )
                        success = prop.record_submission(proposal_id)
                        revlogger.info("%s", success)
                    except Exception:
                        fail = prop.record_malfunction()
                        revlogger.error("%s", fail)
                        raise

            with phase("forum post update"):
                rclogger.debug("Updating forum posts after processing versions.")
                # Update the forum posts in case the proposal was created.
                rc_forum_topic.update(
                    summary_retriever=self.loader.proposal_summary,
                    proposal_id_retriever=self.state.version_proposal,
                )

        if versions:
            logger.info("Iteration completed. %s releases processed.", len(versions))


dre_repo = "dfinity/dre"


def main() -> None:
    parser = argparse.ArgumentParser(
        formatter_class=argparse.ArgumentDefaultsHelpFormatter
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        dest="dry_run",
        help="Make no changes anywhere, including but not limited to proposals, forum posts, or Google documents.",
    )
    parser.add_argument(
        "--commit-annotator-url",
        dest="commit_annotator_url",
        type=str,
        default="http://localhost:9469/",
        help="Base URL of a commit annotator to use in order to determine commit"
             " relevance for a target when composing release notes.  If the string"
             " 'local' is specified, it retrieves annotations using an embedded client"
             " that consults a local Git repository clone of the IC; local mode allows"
             " running the release controller without a commit annotator running"
             " simultaneously on this computer.",
    )
    parser.add_argument(
        "--verbose",
        "--debug",
        action="store_true",
        dest="verbose",
        help="Bump log level.",
    )
    parser.add_argument(
        "--one-line-logs",
        action="store_true",
        dest="one_line_logs",
        help="Make log lines one-line without timestamps (useful in production container for better filtering).",
    )
    parser.add_argument(
        "--loop-every",
        action="store",
        type=int,
        dest="loop_every",
        default=60,
        help="Time to wait (in seconds) between loop executions.  If 0 or less, exit immediately after the first loop.",
    )
    parser.add_argument(
        "--skip-preloading-state",
        action="store_true",
        dest="skip_preloading_state",
        help="Do not fill the reconciler state upon startup with the known proposals from the governance canister.",
    )
    parser.add_argument(
        "--telemetry_port",
        type=int,
        dest="telemetry_port",
        default=9467,
        help="Set the Prometheus telemetry port to listen on.  Telemetry is only served if --loop-every is greater than 0.",
    )
    parser.add_argument(
        "dotenv_file",
        nargs="?",
    )
    opts = parser.parse_args()

    dry_run = opts.dry_run
    skip_preloading_state = opts.skip_preloading_state

    if skip_preloading_state and not dry_run:
        assert 0, "To prevent double submission of proposals, preloading state must not be skipped when run without --dry-run"

    if opts.dotenv_file:
        load_dotenv(opts.dotenv_file)
    else:
        load_dotenv()

    conventional_logging(opts.one_line_logs, opts.verbose)
    logging.getLogger("pydiscourse.client").setLevel(logging.INFO)

    # Prep the program for longer timeouts.
    socket.setdefaulttimeout(60)

    # Watchdog needs to be fed (to report healthy progress) every 10 minutes at the least.
    watchdog = Watchdog(timeout_seconds=max([600, opts.loop_every * 2]))
    watchdog.start()

    config_loader = (
        GitReleaseLoader(f"https://github.com/{dre_repo}.git")
        if "dev" not in os.environ
        else DevReleaseLoader()
    )
    forum_client = (
        ReleaseCandidateForumClient(
            DiscourseClient(  # type: ignore[no-untyped-call]
                host=os.environ["DISCOURSE_URL"],
                api_username=os.environ["DISCOURSE_USER"],
                api_key=os.environ["DISCOURSE_KEY"],
            )
        )
        if not dry_run
        else dryrun.ForumClient(dryrun.StubDiscourseClient())
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
            f"https://oauth2:{os.environ['GITHUB_TOKEN']}@github.com/dfinity/ic.git",
            main_branch="master",
            repo_cache_dir=pathlib.Path.home() / ".cache/reconciler",
        )
        if not dry_run
        else dryrun.GitRepo(
            "https://github.com/dfinity/ic.git",
            main_branch="master",
        )
    )
    publish_notes_client: PublishNotesClientProtocol = (
        PublishNotesClient(
            Github(auth=Auth.Token(os.environ["GITHUB_TOKEN"])).get_repo(dre_repo)
        )
        if not dry_run
        else dryrun.PublishNotesClient()
    )

    dre = (
        dre_cli.DRECli(
            dre_cli.Auth(
                key_path=os.environ["PROPOSER_KEY_FILE"],
                neuron_id=os.environ["PROPOSER_NEURON_ID"],
            ),
        )
        if not dry_run
        else dryrun.DRECli()
    )
    state = reconciler_state.ReconcilerState(
        None if skip_preloading_state else dre.get_election_proposals_by_version,
    )
    slack_announcer: slack_announce.SlackAnnouncerProtocol = (
        slack_announce.SlackAnnouncer() if not dry_run else dryrun.MockSlackAnnouncer()
    )

    def change_determinator_factory() -> ChangeDeterminatorProtocol:
        if opts.commit_annotator_url == "local":
            LOGGER.debug("Using local commit annotator to determine OS changes")
            return LocalCommitChangeDeterminator(ic_repo)
        LOGGER.debug(
            "Using API at %s to determine OS changes", opts.commit_annotator_url
        )
        return CommitAnnotatorClientCommitChangeDeterminator(opts.commit_annotator_url)

    reconciler = Reconciler(
        forum_client=forum_client,
        loader=config_loader,
        notes_client=release_notes_client,
        publish_client=publish_notes_client,
        change_determinator_factory=change_determinator_factory,
        nns_url="https://ic0.app",
        state=state,
        ignore_releases=IGNORED_RELEASES,
        ic_repo=ic_repo,
        active_version_provider=ICPrometheus(
            url="https://victoria.mainnet.dfinity.network/select/0/prometheus"
        ),
        slack_announcer=slack_announcer,
        dre=dre,
    )

    if opts.loop_every > 0:
        start_http_server(port=int(opts.telemetry_port))

    while True:
        try:
            now = time.time()
            LAST_CYCLE_START_TIMESTAMP_SECONDS.set(int(now))
            reconciler.reconcile()
            and_now = time.time()
            LAST_CYCLE_SUCCESS_TIMESTAMP_SECONDS.set(int(and_now))
            LAST_CYCLE_END_TIMESTAMP_SECONDS.set(int(and_now))
            watchdog.report_healthy()
            if opts.loop_every <= 0:
                break
            else:
                sleepytime = opts.loop_every - (time.time() - now)
                if sleepytime > 0.0:
                    time.sleep(sleepytime)
        except KeyboardInterrupt:
            LOGGER.info("Interrupted.")
            raise
        except Exception:
            if opts.loop_every <= 0:
                raise
            else:
                watchdog.report_healthy()
                and_now = time.time()
                LAST_CYCLE_END_TIMESTAMP_SECONDS.set(int(and_now))
                LOGGER.exception(
                    f"Failed to reconcile.  Retrying in {opts.loop_every} seconds.  Traceback:"
                )
                time.sleep(opts.loop_every)

    LOGGER.info("Exiting.")


# use this as a template in case you need to manually submit a proposal
def oneoff() -> None:
    release_loader = GitReleaseLoader(f"https://github.com/{dre_repo}.git")
    version = "ac971e7b4c851b89b312bee812f6de542ed907c5"
    changelog = release_loader.proposal_summary(version, GUESTOS, False)
    assert changelog

    dre = dre_cli.DRECli()
    dre.propose_to_revise_elected_os_versions(
        changelog=changelog,
        version=version,
        os_kind=GUESTOS,
        forum_post_url="https://forum.dfinity.org/t/proposal-to-elect-new-release-rc-2024-03-27-23-01/29042/7",
        unelect_versions=[],
        package_checksum=version_package_checksum(version, GUESTOS),
        package_urls=version_package_urls(version, GUESTOS),
    )


if __name__ == "__main__":
    main()
