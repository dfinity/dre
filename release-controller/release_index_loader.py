import os
import pathlib
import subprocess
import sys
import tempfile

from pydantic_yaml import parse_yaml_raw_as
import release_index
from git_fetcher import GitFetcher
from publish_notes import REPLICA_RELEASES_DIR

RELEASE_INDEX_FILE = "release-index.yaml"


class ReleaseLoader:
    def __init__(self, release_index_dir: pathlib.Path):
        self.release_index_dir = release_index_dir

    def index(self) -> release_index.Model:
        return parse_yaml_raw_as(release_index.Model, open(self.release_index_dir / RELEASE_INDEX_FILE, "r").read())

    def changelog(self, version: str) -> str | None:
        version_changelog_path = self.release_index_dir / f"{REPLICA_RELEASES_DIR}/{version}.md"
        if version_changelog_path.exists():
            return open(version_changelog_path, "r").read()
        return None


class DevReleaseLoader(ReleaseLoader):
    def __init__(self):
        dev_repo_root = (
            subprocess.check_output(["git", "rev-parse", "--show-toplevel"], stderr=subprocess.DEVNULL)
            .decode(sys.stdout.encoding)
            .strip()
        )
        if not dev_repo_root:
            raise RuntimeError("Not running in a dev environment")
        super().__init__(pathlib.Path(dev_repo_root))


class GitReleaseLoader(ReleaseLoader):
    def __init__(self):
        self.git_fetcher = GitFetcher("https://github.com/dfinity/dre.git")
        super().__init__(self.git_fetcher.dir)

    def index(self):
        self.git_fetcher.fetch()
        return super().index()

    def changelog(self, version):
        self.git_fetcher.fetch()
        return super().changelog(version)


class StaticReleaseLoader(ReleaseLoader):
    def __init__(self, config):
        self.tempdir = tempfile.TemporaryDirectory()
        super().__init__(pathlib.Path(self.tempdir.name))
        self.overwrite_config(config)

    def overwrite_config(self, config):
        with open(self.release_index_dir / RELEASE_INDEX_FILE, "w") as f:
            f.write(config)

    def __del__(self):
        self.tempdir.cleanup()
