import argparse
import logging
import os
import subprocess
import sys
import re
import time
import typing
from prometheus_client import start_http_server, Gauge

sys.path.append(os.path.join(os.path.dirname(__file__)))

from git_repo import GitRepo, GitRepoAnnotator, GitRepoBehavior
from datetime import datetime
from tenacity import retry, stop_after_attempt
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


_LOGGER = logging.getLogger()


def release_branch_date(branch: str) -> typing.Optional[datetime]:
    branch_search = re.search(r"rc--(\d{4}-\d{2}-\d{2})", branch, re.IGNORECASE)
    if branch_search:
        branch_date = branch_search.group(1)
    else:
        return None
    return datetime.strptime(branch_date, "%Y-%m-%d")


# target-determinator sometimes fails on first few tries
@retry(stop=stop_after_attempt(10))
def target_determinator(ic_repo: GitRepoAnnotator, object: str) -> bool:
    logger = _LOGGER.getChild("target_determinator").getChild(object)
    logger.debug("Attempting to determine target")
    ic_repo.checkout(object)
    p = subprocess.run(
        [
            resolve_binary("target-determinator"),
            "-before-query-error-behavior=fatal",
            f"-bazel={resolve_binary("bazel")}",
            "--targets",
            GUESTOS_BAZEL_TARGETS,
            ic_repo.parent(object),
        ],
        cwd=ic_repo.dir,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    output = p.stdout.decode().strip()
    logger.debug(f"stdout of target determinator for {object}: '{output}'")
    logger.debug(
        f"stderr of target determinator for {object}: '{p.stderr.decode().strip()}'"
    )
    return output != ""


def annotate_object(ic_repo: GitRepoAnnotator, object: str) -> None:
    logger = _LOGGER.getChild("annotate_object").getChild(object)
    logger.debug("Attempting to annotate")
    ic_repo.checkout(object)
    logger.debug("Running bazel query")
    bazel_query_output = subprocess.check_output(
        [
            resolve_binary("bazel"),
            "query",
            f"deps({GUESTOS_BAZEL_TARGETS})",
        ],
        text=True,
        cwd=ic_repo.dir,
    ).splitlines()
    ic_repo.add(
        object=object,
        namespace=GUESTOS_TARGETS_NOTES_NAMESPACE,
        content="\n".join(
            [
                line
                for line in bazel_query_output
                if line.strip() and not line.startswith("@")
            ]
        ),
    )
    ic_repo.add(
        object=object,
        namespace=GUESTOS_CHANGED_NOTES_NAMESPACE,
        content=str(target_determinator(ic_repo=ic_repo, object=object)),
    )


def annotate_branch(annotator: GitRepoAnnotator, branch: str) -> None:
    logger = _LOGGER.getChild(branch)
    logger.debug("Attempting to annotate")
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
        logger.info("About to annotate %s commits", len(commits))
    for c in reversed(commits):
        annotate_object(ic_repo=annotator, object=c)
    if commits:
        logger.info("Successfully annotated %s commits", len(commits))


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

    # Watchdog needs to be fed (to report healthy progress) every 10 minutes
    watchdog = Watchdog(timeout_seconds=max([600, opts.loop_every * 2]))
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
                    annotate_branch(annotator, branch=b)
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
