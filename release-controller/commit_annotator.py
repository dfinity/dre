import argparse
import http.server
import logging
import os
import pathlib
import shlex
import subprocess
import sys
import re
import threading
import time
import typing
from prometheus_client import start_http_server, Gauge
from tenacity import retry, stop_after_delay, retry_if_exception_type, after_log

sys.path.append(os.path.join(os.path.dirname(__file__)))

from commit_annotation import (
    CHANGED_NOTES_NAMESPACES,
    COMMIT_BELONGS,
    COMMIT_DOES_NOT_BELONG,
    CommitInclusionState,
    GitRepoAnnotator,
)
from git_repo import GitRepo
from datetime import datetime
from util import conventional_logging, resolve_binary, DefaultSubcommandArgParser
from watchdog import Watchdog

from const import (
    OsKind,
    GUESTOS,
    HOSTOS,
    OS_KINDS,
)

BAZEL_TARGETS = {
    # All targets that produce the update image for GuestOS.
    GUESTOS: "deps(//ic-os/guestos/envs/prod:update-img.tar.zst)",
    # All targets that produce the HostOS disk image united with the targets
    # of the SetupOS disk image minus HostOS and GuestOS disk images.
    HOSTOS: """
    deps(
        //ic-os/hostos/envs/prod:disk-img.tar.zst
    ) union (
        deps(
            //ic-os/setupos/envs/prod:disk-img.tar.zst
        ) except deps(
            //ic-os/hostos/envs/prod:disk-img.tar.zst
        ) except deps(
            //ic-os/guestos/envs/prod:disk-img.tar.zst
        ) except //ic-os/setupos/envs/prod/... except //ic-os/setupos/envs/prod:guest-os.img.tar.zst
    )
    """,
}
BAZEL_OPTS: list[str] = [
    "--experimental_build_event_upload_strategy=local",
    "--noremote_upload_local_results",
    "--bes_backend=",
    "--nobes_lifecycle_events",
]
TARGETS_NOTES_NAMESPACES = {GUESTOS: "guestos-targets", HOSTOS: "hostos-targets"}
# One commit before last manual HostOS release performed by Manuel Amador.
CUTOFF_COMMIT = "6f3739270268208945648cc70d8010bda753e827"
# List of branches to ignore from annotation (before the cutoff commit).

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
)
COMMITS_BEHIND = Gauge(
    "commits_behind",
    "Number of commits this ref to be annotated is behind.  On a well-oiled annotator, this never stays above 0 for long.",
    ["ref"],
)


_LOGGER = logging.getLogger()


def release_branch_date(branch: str) -> typing.Optional[datetime]:
    branch_search = re.search(r"rc--(\d{4}-\d{2}-\d{2})", branch, re.IGNORECASE)
    if branch_search:
        branch_date = branch_search.group(1)
    else:
        return None
    return datetime.strptime(branch_date, "%Y-%m-%d")


# target-determinator sometimes fails on first few tries
# we will therefore blow up after 180 seconds
@retry(
    stop=stop_after_delay(180),
    retry=retry_if_exception_type(subprocess.CalledProcessError),
    after=after_log(_LOGGER, logging.ERROR),
)
def target_determinator(
    cwd: pathlib.Path, parent_object: str, bazel_targets: str
) -> str:
    logger = _LOGGER.getChild("target_determinator").getChild(parent_object)
    p = subprocess.run(
        [
            resolve_binary("target-determinator"),
            "-before-query-error-behavior=fatal",
            "-delete-cached-worktree",
            "-bazel-opts=" + " ".join(shlex.quote(x) for x in BAZEL_OPTS),
            f"-bazel={resolve_binary("bazel")}",
            "--targets",
            bazel_targets,
            parent_object,
        ],
        cwd=cwd,
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    )
    output = p.stdout.strip()
    if output:
        logger.debug(
            "stdout of target determinator for %s has %s lines",
            parent_object,
            len(output.splitlines()),
        )
    else:
        logger.debug("stdout of target determinator for %s is empty", parent_object)
    return output


def compute_annotations_for_object(
    annotator: GitRepoAnnotator, object: str, os_kind: OsKind
) -> tuple[str, str, CommitInclusionState]:
    logger = _LOGGER.getChild("annotate_object").getChild(object).getChild(os_kind)
    logger.debug("Attempting to annotate")
    start = time.time()
    targets = BAZEL_TARGETS[os_kind]

    # The following two external operations were being run in parallel
    # to speed things up, but it turns out one of them often modifies
    # the working directory, making the other one much slower.  Thus,
    # we run them serially now, and -- in between them -- we clean the
    # repository's working directory.
    annotator.checkout(object)
    bazel_query_output = subprocess.check_output(
        [
            resolve_binary("bazel"),
            "query",
        ]
        + BAZEL_OPTS
        + [
            f"{targets}",
        ],
        text=True,
        cwd=annotator.dir,
    )
    target_determinator_output = target_determinator(
        annotator.dir, annotator.parent(object), targets
    )
    lap = time.time() - start
    logger.debug("Annotation finished in %.2f seconds", lap)
    return (
        "\n".join(
            [
                line
                for line in bazel_query_output.splitlines()
                if line.strip() and not line.startswith("@")
            ]
        ),
        "\n".join(
            [
                line
                for line in target_determinator_output.splitlines()
                if line.strip() and not line.startswith("@")
            ]
        ),
        (COMMIT_BELONGS if target_determinator_output else COMMIT_DOES_NOT_BELONG),
    )


def annotate_object(annotator: GitRepoAnnotator, object: str, os_kind: OsKind) -> None:
    logger = _LOGGER.getChild("annotate_object").getChild(object).getChild(os_kind)
    try:
        targets_notes, affected_targets, belongs = compute_annotations_for_object(
            annotator, object, os_kind
        )
        annotator.add(
            object=object,
            namespace=TARGETS_NOTES_NAMESPACES[os_kind],
            content="# bazel targets acccording to bazel query\n\n"
            + targets_notes
            + "\n\n# affected targets according to target-determinator\n\n"
            + affected_targets,
        )
        annotator.add(
            object=object,
            namespace=CHANGED_NOTES_NAMESPACES[os_kind],
            content=belongs,
        )
    except Exception:
        logger.exception("Annotation failed.  Aborting.")
        raise


def plan_to_annotate_branch(
    annotator: GitRepoAnnotator,
    branch: str,
    skip_checking_commits: dict[OsKind, set[str]],
    max_commit_depth: int,
) -> dict[OsKind, list[str]]:
    """
    Come up with a plan to annotate all commits in a branch.

    Returns a dictionary keyed by OsKind, where each value is a list of
    commits that need to be annotated, ordered from most recent to oldest.
    This ordering is important and it plays a role in the functionality
    of the calling code.
    """
    logger = _LOGGER.getChild(branch)
    logger.debug("Preparing annotation plan")
    commits: dict[OsKind, list[str]] = {}
    annotator.checkout(branch)
    for kind in OS_KINDS:
        current_commit = branch
        commits[kind] = []
        for n in range(max_commit_depth):
            if current_commit == CUTOFF_COMMIT:
                break
            if current_commit not in skip_checking_commits[kind]:
                if annotator.has(
                    namespace=CHANGED_NOTES_NAMESPACES[kind], object=current_commit
                ):
                    logger.debug(
                        "Found wholly annotated %s commit %s", kind, current_commit
                    )
                    break
            commits[kind].append(current_commit)
            current_commit = annotator.parent(current_commit)
        if n == max_commit_depth - 1:
            logger.info(
                "Stopped traveling back at commit number %s (%s)", n + 1, current_commit
            )
    for kind in OS_KINDS:
        if commits[kind]:
            logger.info(
                "Has %s unannotated %s commits to annotate",
                len(set(commits[kind]) - set(skip_checking_commits[kind])),
                kind,
            )
    COMMITS_BEHIND.labels(branch).set(sum(len(cs) for cs in commits.values()))
    return commits


def annotate_commits_of_branch(
    annotator: GitRepoAnnotator,
    branch: str,
    commits_from_newest_to_oldest: list[str],
    os_kind: OsKind,
    watchdog: Watchdog,
    push_annotations: bool,
) -> None:
    """
    Annotates the commits of a branch.

    The list of commits passed to this function needs to be in newest
    to oldest order, and this function will operate on it in reverse
    fashion, so that a restart of the annotator does not confuse the
    `plan_to_annotate_branch()` function, which discovers commits to
    annotate by walking the commit list backwards from newest to
    oldest until it detects an annotated commit.
    """
    logger = _LOGGER.getChild(branch).getChild(os_kind)
    unannotated_commits = len(commits_from_newest_to_oldest)
    if commits_from_newest_to_oldest:
        # Reverse to annotate oldest objects first so that loop
        # can be easily restarted if it breaks.
        for c in reversed(commits_from_newest_to_oldest):
            annotate_object(annotator=annotator, object=c, os_kind=os_kind)
            if push_annotations:
                # Eagerly push each successful annotation to ensure that
                # the commit annotator can continue where it left off upon restart,
                # and also to ensure that manual interventions such as annotations
                # by a person don't end up having to be "annotate a whole branch worth
                # of commits" kinda work.
                annotator.push()
            unannotated_commits = unannotated_commits - 1
            COMMITS_BEHIND.labels(branch).set(unannotated_commits)
            watchdog.report_healthy()
        logger.info(
            "Successfully annotated %s commits", len(commits_from_newest_to_oldest)
        )
    else:
        COMMITS_BEHIND.labels(branch).set(0)


class APIHandler(http.server.BaseHTTPRequestHandler):
    server: "APIServer"

    def do_GET(self) -> None:
        m = re.match(
            r"/api/v1/commit/([0-9a-f]{6,64})/annotation/(%s|%s)"
            % (
                CHANGED_NOTES_NAMESPACES[GUESTOS],
                CHANGED_NOTES_NAMESPACES[HOSTOS],
            ),
            self.path,
        )
        if m:
            commit, namespace = m.group(1), m.group(2)
            try:
                data = self.server.annotator.get(namespace=namespace, object=commit)
                self.send_response(code=200, message="OK")
                self.send_header("Content-Type", "text/plain")
                self.send_header("Content-Length", f"{len(data)}")
                self.end_headers()
                self.wfile.write(data)
            except KeyError:
                msg = f"No {namespace} annotation for commit {commit}"
                self.send_response(code=404, message=msg)
                self.send_header("Content-Type", "text/plain")
                self.end_headers()
                self.wfile.write(msg.encode("utf-8"))
            except Exception as e:
                self.log_error(
                    "%s while retrieving %s annotation on commit %s: %s",
                    e.__class__.__name__,
                    namespace,
                    commit,
                    e,
                )
                self.send_response(code=500, message=str(e.__class__.__name__))
                self.send_header("Content-Type", "text/plain")
                self.end_headers()
                self.wfile.write(str(e).encode("utf-8"))
                return
        else:
            self.send_response(code=404, message="Endpoint not found")
            self.end_headers()


class APIServer(http.server.HTTPServer):
    def __init__(self, address: typing.Any, annotator: GitRepoAnnotator):
        http.server.HTTPServer.__init__(self, address, APIHandler)
        self.annotator = annotator


def start_api_server(annotator: GitRepoAnnotator, port: int) -> None:
    httpd = APIServer(("", port), annotator)
    t = threading.Thread(target=httpd.serve_forever, daemon=True)
    t.start()


def main() -> None:
    parser = DefaultSubcommandArgParser(
        formatter_class=argparse.ArgumentDefaultsHelpFormatter
    )
    ann_behavior = parser.add_argument_group(title="Annotation behavior")
    ann_behavior.add_argument(
        "--no-save-annotations",
        action="store_false",
        dest="save_annotations",
        help="The default is to save annotations locally.  This option turns it off.  The upside is that, on every loop, the annotator attempts to re-annotate the same commits it had annotated before.",
    )
    ann_behavior.add_argument(
        "--no-fetch-annotations",
        action="store_false",
        dest="fetch_annotations",
        help="The default is to fetch annotations from the repository on every loop, overwriting local annotations. This option turns off that behavior.",
    )
    ann_behavior.add_argument(
        "--no-push-annotations",
        action="store_false",
        dest="push_annotations",
        help="The default is to push annotations to remote server.  This option turns it off.",
    )
    ann_behavior.add_argument(
        "--github-token",
        action="store_true",
        dest="github_token",
        default=None,
        help="Which Github token to use in order to push annotations (defaults to the contents"
        " of the GITHUB_TOKEN environment variable).  Not necessary, but if unset, specify"
        " --no-push-annotations to skip pushing.",
    )
    logging_opts = parser.add_argument_group(title="Logging options")
    logging_opts.add_argument(
        "--verbose",
        "--debug",
        action="store_true",
        dest="verbose",
        help="Bump log level.",
    )
    logging_opts.add_argument(
        "--one-line-logs",
        action="store_true",
        dest="one_line_logs",
        help="Make log lines one-line without timestamps (useful in production container for better filtering).",
    )
    subparsers = parser.add_subparsers(
        title="This program supports several subcommands", dest="cmd"
    )
    run_opts = subparsers.add_parser(
        "run",
        help=(
            "Run the annotator in service mode (the default).  See run --help"
            " for options influencing the behavior of the annotator service"
        ),
    )
    run_opts.add_argument(
        "--loop-every",
        action="store",
        type=int,
        dest="loop_every",
        default=30,
        help="Time to wait (in seconds) between loop executions.  If 0 or less, exit immediately after the first loop.",
    )
    run_opts.add_argument(
        "--watchdog-timer",
        action="store",
        type=int,
        dest="watchdog_timer",
        default=1200,
        help="Kill the annotator if a loop has not completed in this many seconds.",
    )
    run_opts.add_argument(
        "--branch-globs",
        default="master,rc--*",
        type=str,
        dest="branch_globs",
        help="Use this branch glob (or comma-separated list of globs) to determine which branches to annotate.",
    )
    run_opts.add_argument(
        "--skip-branch-older-than",
        default=20,
        type=int,
        dest="max_branch_age_days",
        help="Skip annotating branches older than this value (in days).",
    )
    run_opts.add_argument(
        "--max-commit-depth",
        default=500,
        type=int,
        dest="max_commit_depth",
        help="Maximum number of commits to annotate starting from each branch and going back.",
    )
    run_opts.add_argument(
        "--api-port",
        type=int,
        dest="api_port",
        default=9469,
        help="Port for API service to retrieve annotations.  Only served if --loop-every is greater than 0.  Disabled if less than 1.",
    )
    run_opts.add_argument(
        "--telemetry-port",
        type=int,
        dest="telemetry_port",
        default=9468,
        help="Set the Prometheus telemetry port to listen on.  Telemetry is only served if --loop-every is greater than 0.  Disabled if less than 1.",
    )
    manual_annotation_opts = subparsers.add_parser(
        "manually-annotate",
        help=(
            "Annotate a commit ID as either affecting HostOS/GuestOS (see flag --os-kind)"
            " or not affecting it.  This option takes two arguments: the commit ID followed"
            " by a boolean indicating whether the commit ID affects the OS specified.  Use"
            " of this option suppresses the execution of the normal annotation loop, and"
            " conflicts with most other options intended to be used in it (general options"
            " like --verbose or --no-push-annotations are honored)."
            " If pushing annotations is desired (the default) then ensure to set a"
            " --github-token with write access to the IC repository."
        ),
    )
    manual_annotation_opts.add_argument(
        "--os-kind",
        help="Which OS kind to manually annotate this commit for (one of %s, default all)."
        % OS_KINDS,
        choices=OS_KINDS,
    )
    manual_annotation_opts.add_argument(
        "COMMIT_ID",
        help="The commit ID to annotate.",
        type=str,
    )
    manual_annotation_opts.add_argument(
        "AFFECTED",
        help="Whether the commit ID is affected.",
        choices=["yes", "no"],
    )
    parser.set_default_subparser("run")
    opts = parser.parse_args()

    github_token = opts.github_token or os.getenv("GITHUB_TOKEN", None)
    github_org = os.getenv("GITHUB_ORG", "dfinity")
    creds = f"{github_token}@" if github_token else ""

    conventional_logging(opts.one_line_logs, opts.verbose)

    ic_repo = GitRepo(
        f"https://{creds}github.com/{github_org}/ic.git",
        main_branch="master",
        repo_cache_dir=pathlib.Path.home() / ".cache/commit_annotator",
    )
    annotator = GitRepoAnnotator(
        ic_repo,
        list(TARGETS_NOTES_NAMESPACES.values())
        + list(CHANGED_NOTES_NAMESPACES.values()),
        opts.save_annotations,
    )

    logger = _LOGGER.getChild("annotator")

    if opts.cmd == "run":
        branch_globs = opts.branch_globs.split(",")
        # Watchdog needs to be fed (to report healthy progress) every watchdog_timer seconds
        watchdog = Watchdog(timeout_seconds=opts.watchdog_timer)
        watchdog.start()

        if opts.loop_every > 0:
            if int(opts.telemetry_port) > 0:
                start_http_server(port=int(opts.telemetry_port))
            if int(opts.api_port) > 0:
                start_api_server(annotator, port=int(opts.api_port))

        while True:
            try:
                now = time.time()
                LAST_CYCLE_START_TIMESTAMP_SECONDS.set(int(now))
                ic_repo.fetch()

                if opts.fetch_annotations:
                    annotator.fetch()

                # Performance optimization to avoid calling git notes on every
                # commit once per branch.  Should only need to call it once.
                commits_to_annotate: dict[OsKind, set[str]] = {
                    k: set() for k in OS_KINDS
                }
                unannotated_commits_by_ref: dict[str, dict[OsKind, list[str]]] = {}
                for b in [
                    branch
                    for glob in branch_globs
                    for branch in ic_repo.branch_list(glob)
                ]:
                    # if b is a directly-specified branch instead of a glob
                    # then assume the date is "now" rather than fool around
                    # with trying to determine the branch date.
                    branch_date = (
                        datetime.now() if b in branch_globs else release_branch_date(b)
                    )
                    if (
                        not branch_date
                        or (datetime.now() - branch_date).days
                        > opts.max_branch_age_days
                    ):
                        logger.debug(
                            "Ignoring branch as older than %s days: %s",
                            opts.max_branch_age_days,
                            b,
                        )
                        continue
                    outstanding_commits = plan_to_annotate_branch(
                        annotator,
                        branch=b,
                        skip_checking_commits=commits_to_annotate,
                        max_commit_depth=opts.max_commit_depth,
                    )
                    unannotated_commits_by_ref[b] = outstanding_commits
                    for k, v in commits_to_annotate.items():
                        v.update(outstanding_commits[k])
                # Make a little cache for this loop's run, saving time not invoking
                # the annotation for a simgle commit twice.  Git history of different
                # branches often shares commits in both branches.
                annotated_commits: dict[OsKind, set[str]] = {k: set() for k in OS_KINDS}
                for b, kinds_and_commits in unannotated_commits_by_ref.items():
                    # Remove any commits already annotated so as to not waste time,
                    # but preserve the ordering since that seems (to me) to matter.
                    for kind, outstanding_commits_by_kind in kinds_and_commits.items():
                        tbd = [
                            c
                            for c in outstanding_commits_by_kind
                            if c not in annotated_commits[kind]
                        ]
                        annotate_commits_of_branch(
                            annotator, b, tbd, kind, watchdog, opts.push_annotations
                        )
                        # Remember these were annotated, avoid wasting time next loop.
                        annotated_commits[kind].update(tbd)

                and_now = time.time()
                LAST_CYCLE_SUCCESS_TIMESTAMP_SECONDS.set(int(and_now))
                LAST_CYCLE_END_TIMESTAMP_SECONDS.set(int(and_now))
                LAST_CYCLE_SUCCESSFUL.set(1)
                watchdog.report_healthy()
                if opts.loop_every <= 0:
                    break
                else:
                    sleepytime = opts.loop_every - (time.time() - now)
                    if sleepytime > 0.0:
                        time.sleep(sleepytime)
            except KeyboardInterrupt:
                logger.info("Interrupted.")
                raise
            except Exception:
                if opts.loop_every <= 0:
                    raise
                else:
                    watchdog.report_healthy()
                    and_now = time.time()
                    LAST_CYCLE_END_TIMESTAMP_SECONDS.set(int(and_now))
                    LAST_CYCLE_SUCCESSFUL.set(0)
                    logger.exception(
                        f"Failed to annotate.  Retrying in {opts.loop_every} seconds.  Traceback:"
                    )
                    time.sleep(opts.loop_every)
    elif opts.cmd == "manually-annotate":
        ic_repo.fetch()
        if opts.fetch_annotations:
            logger.info("Fetching annotations in preparation for manual annotation.")
            annotator.fetch()
        else:
            logger.warning(
                "Not fetching annotations per request.  This may"
                " cause push to fail if/when push is attempted."
            )

        os_kinds = [opts.os_kind] if opts.os_kind else OS_KINDS
        commit_id = opts.COMMIT_ID
        affected = True if opts.AFFECTED == "yes" else False
        for os_kind in os_kinds:
            val = COMMIT_BELONGS if affected else COMMIT_DOES_NOT_BELONG
            logger.info(
                "Manually annotating %s at commit %s as %s (%r) by the commit.",
                os_kind,
                commit_id,
                "affected" if affected else "not affected",
                val,
            )
            annotator.checkout(commit_id)
            annotator.add(
                object=commit_id,
                namespace=TARGETS_NOTES_NAMESPACES[os_kind],
                content="# This commit was manually annotated by commit-annotator manually-annotate command.",
            )
            annotator.add(
                object=commit_id,
                namespace=CHANGED_NOTES_NAMESPACES[os_kind],
                content=val,
            )
            if opts.push_annotations:
                if not github_token:
                    logger.warning(
                        "About to push annotations, but no --github-token set.  This may fail or hang!"
                    )
                # Eagerly push each successful annotation to ensure that
                # the commit annotator can continue where it left off upon restart,
                # and also to ensure that manual interventions such as annotations
                # by a person don't end up having to be "annotate a whole branch worth
                # of commits" kinda work.
                logger.info("Pushing annotations after manual commit annotation.")
                annotator.push()


if __name__ == "__main__":
    main()
