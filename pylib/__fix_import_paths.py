import subprocess
import sys
import os

if os.environ['BAZEL'] != 'true':
    repo_root = (
        subprocess.check_output(["git", "rev-parse", "--show-superproject-working-tree", "--show-toplevel"])
        .decode("utf-8")
        .splitlines()[0]
    )
    sys.path.append(repo_root)
