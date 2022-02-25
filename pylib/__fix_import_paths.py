import subprocess
import sys

repo_root = subprocess.check_output(["git", "rev-parse", "--show-toplevel"]).decode("utf-8").rstrip()
sys.path.append(repo_root)
