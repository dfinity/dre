import argparse
import concurrent.futures
import logging
import os
import pathlib
import subprocess
import sys
import re
import time
import typing
from prometheus_client import start_http_server, Gauge

sys.path.append(os.path.join(os.path.dirname(__file__)))

from git_repo import GitRepo, GitRepoAnnotator, GitRepoBehavior
from datetime import datetime
from tenacity import retry, stop_after_delay, retry_if_exception_type, after_log
from util import resolve_binary, conventional_logging
from watchdog import Watchdog

from const import GUESTOS_CHANGED_NOTES_NAMESPACE

GUESTOS_TARGETS_NOTES_NAMESPACE = "guestos-targets"
GUESTOS_BAZEL_TARGETS = "//ic-os/guestos/envs/prod:update-img.tar.zst union //ic-os/setupos/envs/prod:disk-img.tar.zst"
CUTOFF_COMMIT = "8646665552677436c8a889ce970857e531fee49b"

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
def target_determinator(cwd: pathlib.Path, parent_object: str) -> bool:
    logger = _LOGGER.getChild("target_determinator").getChild(parent_object)
    p = subprocess.run(
        [
            resolve_binary("target-determinator"),
            "-before-query-error-behavior=fatal",
            f"-bazel={resolve_binary("bazel")}",
            "--targets",
            GUESTOS_BAZEL_TARGETS,
            parent_object,
        ],
        cwd=cwd,
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    )
    output = p.stdout.strip()
    logger.debug(
        f"stdout of target determinator for {parent_object}: %s",
        output,
    )
    return output != ""


def annotate_object(ic_repo: GitRepoAnnotator, object: str) -> None:
    logger = _LOGGER.getChild("annotate_object").getChild(object)
    logger.debug("Attempting to annotate")
    start = time.time()
    ic_repo.checkout(object)

    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
        logger.debug(f"Running bazel query deps({GUESTOS_BAZEL_TARGETS})")
        bazel_query_output_future = executor.submit(
            subprocess.check_output,
            [
                resolve_binary("bazel"),
                "query",
                f"deps({GUESTOS_BAZEL_TARGETS})",
            ],
            text=True,
            cwd=ic_repo.dir,
        )
        target_determinator_future = executor.submit(
            target_determinator, ic_repo.dir, ic_repo.parent(object)
        )
        bazel_query_output = bazel_query_output_future.result()
        lap = time.time() - start
        logger.debug("Bazel query finished in %.2f seconds", lap)
        target_determinator_output = target_determinator_future.result()
        lap = time.time() - start
        logger.debug("Target determinator finished in %.2f seconds", lap)

    ic_repo.add(
        object=object,
        namespace=GUESTOS_TARGETS_NOTES_NAMESPACE,
        content="\n".join(
            [
                line
                for line in bazel_query_output.splitlines()
                if line.strip() and not line.startswith("@")
            ]
        ),
    )
    ic_repo.add(
        object=object,
        namespace=GUESTOS_CHANGED_NOTES_NAMESPACE,
        content=str(target_determinator_output),
    )
    lap = time.time() - start
    logger.debug("Annotation finished in %.2f seconds", lap)


def plan_to_annotate_branch(annotator: GitRepoAnnotator, branch: str) -> list[str]:
    logger = _LOGGER.getChild(branch)
    logger.info("Preparing annotation plan")
    # Reverse to annotate oldest objects first so that loop can be easily restarted if it breaks.
    commits = []
    annotator.checkout(branch)
    current_commit = branch
    while True:
        if current_commit == CUTOFF_COMMIT:
            break
        if annotator.get(
            namespace=GUESTOS_CHANGED_NOTES_NAMESPACE, object=current_commit
        ):
            logger.debug("Found annotated commit %s", current_commit)
            break
        logger.debug("Found unannotated commit %s", current_commit)
        commits.append(current_commit)
        current_commit = annotator.parent(current_commit)
    if commits:
        logger.info("Has %s commits to annotate", len(commits))
    COMMITS_BEHIND.labels(branch).set(len(commits))
    return commits


def annotate_commits_of_branch(
    annotator: GitRepoAnnotator, branch: str, commits: list[str]
) -> None:
    logger = _LOGGER.getChild(branch)
    unannotated_commits = len(commits)
    if commits:
        for c in reversed(commits):
            annotate_object(ic_repo=annotator, object=c)
            unannotated_commits = unannotated_commits - 1
            COMMITS_BEHIND.labels(branch).set(unannotated_commits)
        logger.info("Successfully annotated %s commits", len(commits))
    else:
        COMMITS_BEHIND.labels(branch).set(0)


def main() -> None:
    parser = argparse.ArgumentParser(
        formatter_class=argparse.ArgumentDefaultsHelpFormatter
    )
    parser.add_argument(
        "--no-push-annotations",
        action="store_false",
        dest="push_annotations",
        help="The default is to push annotations to remote server.  This option turns it off.",
    )
    parser.add_argument(
        "--no-save-annotations",
        action="store_false",
        dest="save_annotations",
        help="The default is to save annotations locally.  This option turns it off.  The upside is that, on every loop, the annotator attempts to re-annotate the same commits it had annotated before",
    )
    parser.add_argument(
        "--no-fetch-annotations",
        action="store_false",
        dest="fetch_annotations",
        help="The default is to fetch annotations from the repository on every loop, overwriting local annotations. This option turns off that behavior.",
    )
    parser.add_argument("--verbose", "--debug", action="store_true", dest="verbose")
    parser.add_argument(
        "--one-line-logs",
        action="store_true",
        dest="one_line_logs",
        help="Make log lines one-line without timestamps (useful in production container for better filtering)",
    )
    parser.add_argument(
        "--loop-every",
        action="store",
        type=int,
        dest="loop_every",
        default=30,
        help="Time to wait (in seconds) between loop executions.  If 0 or less, exit immediately after the first loop.",
    )
    parser.add_argument(
        "--watchdog-timer",
        action="store",
        type=int,
        dest="watchdog_timer",
        default=1200,
        help="Kill the annotator if a loop has not completed in this many seconds.",
    )
    parser.add_argument(
        "--branch-globs",
        default="master,rc--*",
        type=str,
        dest="branch_globs",
        help="Use this branch glob (or comma-separated list of globs) to determine which branches to annotate",
    )
    parser.add_argument(
        "--skip-branch-older-than",
        default=20,
        type=int,
        dest="max_branch_age_days",
        help="Skip annotating branches older than this value (in days).",
    )
    parser.add_argument(
        "--telemetry_port",
        type=int,
        dest="telemetry_port",
        default=9468,
        help="Set the Prometheus telemetry port to listen on.  Telemetry is only served if --loop-every is greater than 0.",
    )
    opts = parser.parse_args()

    behavior: GitRepoBehavior = {
        "push_annotations": opts.push_annotations,
        "save_annotations": opts.save_annotations,
        "fetch_annotations": opts.fetch_annotations,
    }
    github_token = os.getenv("GITHUB_TOKEN", None)
    github_org = os.getenv("GITHUB_ORG", "dfinity")
    creds = f"oauth2:{github_token}@" if github_token else ""

    conventional_logging(opts.one_line_logs, opts.verbose)

    ic_repo = GitRepo(
        f"https://{creds}github.com/{github_org}/ic.git",
        main_branch="master",
        behavior=behavior,
    )

    # Watchdog needs to be fed (to report healthy progress) every watchdog_timer seconds
    watchdog = Watchdog(timeout_seconds=opts.watchdog_timer)
    watchdog.start()

    logger = _LOGGER.getChild("annotator")
    branch_globs = opts.branch_globs.split(",")

    if opts.loop_every > 0:
        start_http_server(port=int(opts.telemetry_port))

    while True:
        try:
            now = time.time()
            LAST_CYCLE_START_TIMESTAMP_SECONDS.set(int(now))
            ic_repo.fetch()
            with ic_repo.annotator(
                [GUESTOS_CHANGED_NOTES_NAMESPACE, GUESTOS_TARGETS_NOTES_NAMESPACE]
            ) as annotator:
                unannotated_commits_by_ref: dict[str, list[str]] = {}
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
                    outstanding_commits = plan_to_annotate_branch(annotator, branch=b)
                    unannotated_commits_by_ref[b] = outstanding_commits
                # Make a little cache for this loop's run, saving time not invoking
                # the annotation for a simgle commit twice.  Git history of different
                # branches often shares commits in both branches.
                annotated_commits: set[str] = set()
                for b, outstanding_commits in unannotated_commits_by_ref.items():
                    # Remove any commits already annotated so as to not waste time,
                    # but preserve the ordering since that seems (to me) to matter.
                    tbd = [c for c in outstanding_commits if c not in annotated_commits]
                    annotate_commits_of_branch(annotator, b, tbd)
                    # Remember these were annotated, avoid wasting time next loop.
                    annotated_commits.update(tbd)
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
                and_now = time.time()
                LAST_CYCLE_END_TIMESTAMP_SECONDS.set(int(and_now))
                LAST_CYCLE_SUCCESSFUL.set(0)
                logger.exception(
                    f"Failed to annotate.  Retrying in {opts.loop_every} seconds.  Traceback:"
                )
                time.sleep(opts.loop_every)


if __name__ == "__main__":
    main()
