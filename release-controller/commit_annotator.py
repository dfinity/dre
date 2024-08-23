import logging
import os
import subprocess
import sys
import re
sys.path.append(os.path.join(os.path.dirname(__file__)))

from git_repo import GitRepo
from datetime import datetime
from tenacity import retry, stop_after_attempt
from util import bazel_binary


GUESTOS_CHANGED_NOTES_NAMESPACE = "guestos-changed"
GUESTOS_TARGETS_NOTES_NAMESPACE = "guestos-targets"
GUESTOS_BAZEL_TARGETS = "//ic-os/guestos/envs/prod:update-img.tar.zst union //ic-os/setupos/envs/prod:disk-img.tar.zst"
CUTOFF_COMMIT = "8646665552677436c8a889ce970857e531fee49b"


def release_branch_date(branch: str) -> datetime:
    branch_search = re.search(r"rc--(\d{4}-\d{2}-\d{2})", branch, re.IGNORECASE)
    if branch_search:
        branch_date = branch_search.group(1)
    else:
        raise Exception(f"branch '{branch}' does not match RC branch format")
    return datetime.strptime(branch_date, "%Y-%m-%d")


# target-determinator sometimes fails on first few tries
@retry(stop=stop_after_attempt(10))
def target_determinator(ic_repo: GitRepo, object: str) -> bool:
    ic_repo.checkout(object)
    target_determinator_binary = "target-determinator"
    target_determinator_binary_local = os.path.join(os.path.dirname(__file__), "target-determinator")
    if os.path.exists(target_determinator_binary_local):
        target_determinator_binary = target_determinator_binary_local

    p = subprocess.run(
        [
            target_determinator_binary,
            "-before-query-error-behavior=fatal",
            f"-bazel={bazel_binary()}",
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
    logging.info(f"stderr of target determinator for {object}: '{p.stderr.decode().strip()}'")
    return output != ""


def annotate_object(ic_repo: GitRepo, object: str):
    logging.info("annotating {}".format(object))
    ic_repo.checkout(object)
    ic_repo.add_note(
        GUESTOS_TARGETS_NOTES_NAMESPACE,
        object=object,
        content="\n".join(
            [
                l
                for l in subprocess.check_output(["bazel", "query", f"deps({GUESTOS_BAZEL_TARGETS})"], cwd=ic_repo.dir)
                .decode()
                .splitlines()
                if not l.startswith("@")
            ]
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
        if ic_repo.get_note(namespace=GUESTOS_CHANGED_NOTES_NAMESPACE, object=current_commit):
            break

        logging.info("will annotate {}".format(current_commit))
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
    while True:
        ic_repo.fetch()
        for b in ic_repo.branch_list("rc--*"):
            if (datetime.now() - release_branch_date(b)).days > 20:
                logging.info("skipping branch {}".format(b))
                continue
            logging.info("annotating branch {}".format(b))
            annotate_branch(ic_repo, branch=b)


if __name__ == "__main__":
    logging.basicConfig(stream=sys.stdout, level=logging.INFO)
    main()
