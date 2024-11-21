import logging
import os
import subprocess
import sys
import re
import typing

sys.path.append(os.path.join(os.path.dirname(__file__)))

from git_repo import GitRepo
from datetime import datetime
from tenacity import retry, stop_after_attempt
from util import resolve_binary
from watchdog import Watchdog


GUESTOS_CHANGED_NOTES_NAMESPACE = "guestos-changed"
GUESTOS_TARGETS_NOTES_NAMESPACE = "guestos-targets"
GUESTOS_BAZEL_TARGETS = "//ic-os/guestos/envs/prod:update-img.tar.zst union //ic-os/setupos/envs/prod:disk-img.tar.zst"
CUTOFF_COMMIT = "8646665552677436c8a889ce970857e531fee49b"


def release_branch_date(branch: str) -> typing.Optional[datetime]:
    branch_search = re.search(r"rc--(\d{4}-\d{2}-\d{2})", branch, re.IGNORECASE)
    if branch_search:
        branch_date = branch_search.group(1)
    else:
        return None
    return datetime.strptime(branch_date, "%Y-%m-%d")


# target-determinator sometimes fails on first few tries
@retry(stop=stop_after_attempt(10))
def target_determinator(ic_repo: GitRepo, object: str) -> bool:
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
    logging.info(f"stdout of target determinator for {object}: '{output}'")
    logging.info(
        f"stderr of target determinator for {object}: '{p.stderr.decode().strip()}'"
    )
    return output != ""


def annotate_object(ic_repo: GitRepo, object: str):
    logging.info("annotating git commit {}".format(object))
    ic_repo.checkout(object)
    logging.info("running bazel query for {}".format(object))
    bazel_query_output = (
        subprocess.check_output(
            [
                resolve_binary("bazel"),
                "query",
                f"deps({GUESTOS_BAZEL_TARGETS})",
            ],
            cwd=ic_repo.dir,
        )
        .decode()
        .splitlines()
    )
    ic_repo.add_note(
        GUESTOS_TARGETS_NOTES_NAMESPACE,
        object=object,
        content="\n".join(
            [line for line in bazel_query_output if not line.startswith("@")]
        ),
    )
    ic_repo.add_note(
        namespace=GUESTOS_CHANGED_NOTES_NAMESPACE,
        object=object,
        content=str(target_determinator(ic_repo=ic_repo, object=object)),
    )


def annotate_branch(ic_repo: GitRepo, branch: str):
    commits = []
    current_commit = branch
    ic_repo.checkout(branch)
    while True:
        if current_commit == CUTOFF_COMMIT:
            break
        if ic_repo.get_note(
            namespace=GUESTOS_CHANGED_NOTES_NAMESPACE, object=current_commit
        ):
            break

        logging.info("branch %s: found git commit %s", branch, current_commit)
        commits.append(current_commit)
        current_commit = ic_repo.parent(current_commit)

    # reverse to annotate oldest objects first so that loop can be easily restarted if it breaks
    for c in reversed(commits):
        annotate_object(ic_repo=ic_repo, object=c)


def main():
    ic_repo = GitRepo(
        f"https://oauth2:{os.environ['GITHUB_TOKEN']}@github.com/{os.environ['GITHUB_ORG']}/ic.git",
        main_branch="master",
    )

    # Watchdog needs to be fed (to report healthy progress) every 10 minutes
    watchdog = Watchdog(timeout_seconds=600)
    watchdog.start()

    while True:
        ic_repo.fetch()
        try:
            annotate_branch(ic_repo, branch="master")
        except Exception as e:
            logging.error("failed to annotate master, retrying", exc_info=e)
            continue
        for b in ic_repo.branch_list("rc--*"):
            try:
                branch_date = release_branch_date(b)
                if not branch_date or (datetime.now() - branch_date).days > 20:
                    logging.info("skipping git branch as too old: {}".format(b))
                    continue
                logging.info("annotating branch {}".format(b))
                annotate_branch(ic_repo, branch=b)
                watchdog.report_healthy()
            except Exception as e:
                logging.error(
                    "failed to annotate branch {}, will retry later".format(b),
                    exc_info=e,
                )
                continue


if __name__ == "__main__":
    logging.basicConfig(stream=sys.stdout, level=logging.INFO)
    main()
