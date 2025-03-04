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
import __fix_import_paths  # isort:skip  # noqa: F401 # pylint: disable=W0611
import dre_cli
import dryrun
import release_index
import slack_announce
import reconciler_state
import util
from dotenv import load_dotenv
from forum import ReleaseCandidateForumClient, ForumClientProtocol
from git_repo import GitRepo
from github import Auth
from github import Github
from google_docs import ReleaseNotesClient, ReleaseNotesClientProtocol
from governance import GovernanceCanister
from prometheus import ICPrometheus
from prometheus_client import start_http_server, Gauge
from publish_notes import PublishNotesClient, PublishNotesClientProtocol
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
)


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
    longfmt = "%(asctime)s %(levelname)13s  %(message)s\n" "%(name)37s"

    FORMATS = {
        logging.DEBUG: blue + fmt + reset,
        logging.INFO: green + fmt + reset,
        logging.WARNING: yellow + fmt + reset,
        logging.ERROR: red + fmt + reset,
        logging.CRITICAL: bold_red + fmt + reset,
    }

    LONG_FORMATS = {
        logging.DEBUG: blue + longfmt + reset,
        logging.INFO: green + longfmt + reset,
        logging.WARNING: yellow + longfmt + reset,
        logging.ERROR: red + longfmt + reset,
        logging.CRITICAL: bold_red + longfmt + reset,
    }

    def format(self, record: logging.LogRecord) -> str:
        if len(record.name) > 37 or True:
            log_fmt = self.LONG_FORMATS.get(record.levelno)
        else:
            log_fmt = self.FORMATS.get(record.levelno)
        formatter = logging.Formatter(log_fmt)
        return formatter.format(record)


LOGGER = logging.getLogger()


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


def version_package_urls(version: str) -> list[str]:
    return [
        f"https://download.dfinity.systems/ic/{version}/guest-os/update-img/update-img.tar.zst",
        f"https://download.dfinity.network/ic/{version}/guest-os/update-img/update-img.tar.zst",
    ]


def version_package_checksum(version: str) -> str:
    hashurl = (
        f"https://download.dfinity.systems/ic/{version}/guest-os/update-img/SHA256SUMS"
    )
    response = requests.get(hashurl, timeout=10)
    checksum = typing.cast(
        str,
        [
            line
            for line in response.content.decode("utf-8").splitlines()
            if line.strip().endswith("update-img.tar.zst")
        ][0].split(" ")[0],
    )

    for u in version_package_urls(version):
        LOGGER.getChild("version_package_checksum").debug("fetching package %s", u)
        with requests.get(u, timeout=10, stream=True) as resp:
            resp.raise_for_status()
            actual_sum = util.sha256sum_http_response(
                resp, urllib.parse.urlparse(u).netloc
            )
        if actual_sum != checksum:
            raise ValueError(
                "checksums for %s do not match contents of %s" % (u, hashurl)
            )

    return checksum


class ActiveVersionProvider(typing.Protocol):
    def active_versions(self) -> list[str]: ...


class ReplicaVersionProposalProvider(typing.Protocol):
    def replica_version_proposals(self) -> dict[str, int]: ...


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
        active_version_provider: ActiveVersionProvider,
        replica_version_proposal_provider: ReplicaVersionProposalProvider,
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
        self.governance_canister = replica_version_proposal_provider
        self.state = state
        self.ic_prometheus = active_version_provider
        self.ic_repo = ic_repo
        self.ignore_releases = ignore_releases or []
        self.dre = dre
        self.slack_announcer = slack_announcer

    def reconcile(self) -> None:
        """Reconcile the state of the network with the release index."""
        config = self.loader.index()
        active_versions = self.ic_prometheus.active_versions()
        logger = LOGGER.getChild("reconciler")

        logger.info(
            "GuestOS versions active on subnets or unassigned nodes: %s",
            ", ".join(active_versions),
        )
        releases = config.root.releases[
            : config.root.releases.index(oldest_active_release(config, active_versions))
            + 1
        ]
        if releases:
            logger.info("Dealing with the following releases:")
            for idx, rc in enumerate(releases):
                logger.info(
                    "%s. %s (%s)",
                    idx + 1,
                    rc.rc_name,
                    ", ".join([v.name for v in rc.versions]),
                )
        else:
            logger.info("No releases to deal with")

        for rc in releases:
            rclogger = logger.getChild(f"{rc.rc_name}")

            if rc.rc_name in self.ignore_releases:
                rclogger.debug("In ignore list.  Skipping.")
                continue

            for v_idx, v in enumerate(rc.versions):
                release_tag, release_commit, is_security_fix = (
                    version_name(rc_name=rc.rc_name, name=v.name),
                    v.version,
                    v.security_fix,
                )
                revlogger = rclogger.getChild(f"{release_tag}")

                prop = self.state.version_proposal(release_commit)
                if isinstance(prop, reconciler_state.SubmittedProposal):
                    revlogger.debug("%s.  Nothing to do.", prop)
                    continue
                elif (
                    isinstance(prop, reconciler_state.DREMalfunction)
                    and not prop.ready_to_retry()
                ):
                    revlogger.debug("%s.  Not ready to retry yet.")
                    continue

                # Here we must check, if the proposal has malfunctioned before,
                # that the proposal went through (it may have gone through!)
                # and therefore update posts accordingly (basically everything
                # except actual proposal submission, since it succeeded before
                # despite the failure returned to us by governance canister).

                discovered_proposal: dre_cli.ElectionProposal | None = None
                if isinstance(prop, reconciler_state.DREMalfunction):
                    existing_proposals = self.dre.get_election_proposals_by_version()
                    if discovered_proposal := existing_proposals.get(release_commit):
                        revlogger.warning(
                            "%s.  However, contrary to recorded failure, proposal"
                            " to elect %s was indeed successfully submitted as ID %s.",
                            prop,
                            release_commit,
                            discovered_proposal["id"],
                        )
                    else:
                        revlogger.info("%s.  Retrying process.", prop)
                else:
                    revlogger.info("%s.  Proposal needed.  Beginning process.", prop)

                # update to create posts for any releases
                rclogger.debug("Ensuring forum post for release candidate exists.")
                rc_forum_topic = self.forum_client.get_or_create(rc)

                rclogger.debug("Updating forum post preemptively.")
                rc_forum_topic.update(
                    summary_retriever=self.loader.proposal_summary,
                    proposal_id_retriever=self.state.version_proposal,
                )

                if markdown_file := self.notes_client.markdown_file(release_commit):
                    revlogger.info(
                        "Has release notes in editor.  No need to create them."
                    )
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
                        ) = SecurityReleaseNotesRequest(release_tag, release_commit)
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
                            webhook=os.environ["SLACK_WEBHOOK_URL"],
                            version_name=release_tag,
                            google_doc_url=gdoc["alternateLink"],
                            tag_all_teams=v_idx == 0,
                        )

                self.publish_client.publish_if_ready(
                    google_doc_markdownified=markdown_file,
                    version=release_commit,
                )

                # returns a result only if changelog is published
                changelog = self.loader.proposal_summary(
                    release_commit, is_security_fix
                )
                if not changelog:
                    revlogger.debug("No changelog ready for proposal submission.")
                    continue
                else:
                    revlogger.info(
                        "The changelog is now ready for proposal submission."
                    )

                unelect_versions = []
                if v_idx == 0:
                    unelect_versions.extend(
                        versions_to_unelect(
                            config,
                            active_versions=active_versions,
                            elected_versions=self.dre.get_blessed_versions(),
                        ),
                    )

                if discovered_proposal is not None:
                    prop.record_submission(discovered_proposal["id"])
                else:
                    checksum = version_package_checksum(release_commit)
                    urls = version_package_urls(release_commit)

                    try:
                        proposal_id = (
                            self.dre.propose_to_revise_elected_guestos_versions(
                                changelog=changelog,
                                version=release_commit,
                                forum_post_url=rc_forum_topic.post_url(release_commit),
                                unelect_versions=unelect_versions,
                                package_checksum=checksum,
                                package_urls=urls,
                            )
                        )
                        success = prop.record_submission(proposal_id)
                        revlogger.info("%s", success)
                    except Exception:
                        fail = prop.record_malfunction()
                        revlogger.exception("%s", fail)

                rclogger.debug("Updating forum posts after processing versions.")
                # Update the forum posts in case the proposal was created.
                rc_forum_topic.update(
                    summary_retriever=self.loader.proposal_summary,
                    proposal_id_retriever=self.state.version_proposal,
                )

        logger.info("Iteration completed.")


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
    parser.add_argument("--verbose", "--debug", action="store_true", dest="verbose")
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
    verbose = opts.verbose
    skip_preloading_state = opts.skip_preloading_state

    if skip_preloading_state and not dry_run:
        assert 0, "To prevent double submission of proposals, preloading state must not be skipped when run without --dry-run"

    if opts.dotenv_file:
        load_dotenv(opts.dotenv_file)
    else:
        load_dotenv()

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

    cli_path = pathlib.Path("rs/cli/dre") if os.getenv("BAZEL") == "true" else None
    dre = (
        dre_cli.DRECli(
            dre_cli.Auth(
                key_path=os.environ["PROPOSER_KEY_FILE"],
                neuron_id=os.environ["PROPOSER_NEURON_ID"],
            ),
            cli_path=cli_path,
        )
        if not dry_run
        else dryrun.DRECli(cli_path=cli_path)
    )
    state = reconciler_state.ReconcilerState(
        None if skip_preloading_state else dre.get_election_proposals_by_version,
    )
    slack_announcer = (
        slack_announce.SlackAnnouncer() if not dry_run else dryrun.MockSlackAnnouncer()
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

    if opts.loop_every > 0:
        start_http_server(port=int(opts.telemetry_port))

    while True:
        try:
            now = time.time()
            LAST_CYCLE_START_TIMESTAMP_SECONDS.set(int(time.time()))
            reconciler.reconcile()
            LAST_CYCLE_SUCCESS_TIMESTAMP_SECONDS.set(int(time.time()))
            LAST_CYCLE_SUCCESSFUL.set(1)
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
                LAST_CYCLE_SUCCESSFUL.set(0)
                LOGGER.exception(
                    f"Failed to reconcile.  Retrying in {opts.loop_every} seconds.  Traceback:"
                )
                time.sleep(opts.loop_every)

    LOGGER.info("Exiting.")


# use this as a template in case you need to manually submit a proposal
def oneoff() -> None:
    release_loader = GitReleaseLoader(f"https://github.com/{dre_repo}.git")
    version = "ac971e7b4c851b89b312bee812f6de542ed907c5"
    changelog = release_loader.proposal_summary(version, False)
    assert changelog

    dre = dre_cli.DRECli()
    dre.propose_to_revise_elected_guestos_versions(
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
