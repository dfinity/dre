import difflib
import logging
import os
import shutil
import sys
import tempfile
import pathlib
import typing
import json

from binascii import crc32

me = os.path.join(os.path.dirname(__file__))
if me not in sys.path:
    sys.path.append(me)

import git_repo  # noqa: E402
import dre_cli  # noqa: E402
from const import OsKind, GUESTOS  # noqa: E402
from google_docs import DocInfo  # noqa: E402
from release_notes_composer import PreparedReleaseNotes  # noqa: E402
from release_index import Release  # noqa: E402

import forum  # noqa: E402


LOGGER = logging.getLogger(__name__)


class StubDiscourseClient(object):
    def __init__(self) -> None:
        self.topics: list[forum.Topic] = []
        self.api_username = "doesntmatter"
        self.host = "https://forum.dfinity.org"
        self._logger = LOGGER.getChild(self.__class__.__name__)

        if f := os.environ.get("RECONCILER_DRY_RUN_FORUM_STORAGE"):
            self.forum_storage: pathlib.Path | None = pathlib.Path(f)
            os.makedirs(self.forum_storage, exist_ok=True)
            try:
                with open(self.forum_storage / "mock-posts.json", "rb") as fdata:
                    self.topics = json.load(fdata)
            except FileNotFoundError:
                pass
        else:
            self.forum_storage = None

    def _persist(self) -> None:
        if 0:
            for topic in self.topics:
                self._logger.debug(f"* Topic {topic['id']} titled {topic['title']}")
                for reply in topic["post_stream"]["posts"]:
                    self._logger.debug(
                        f"  * Reply {reply['id']} (topic ID {reply['topic_id']})"
                    )
                    self._logger.debug(
                        f"    Content {reply['raw'].splitlines()[0].strip()}"
                    )
        if self.forum_storage:
            with open(self.forum_storage / "mock-posts.json", "w") as fdata:
                json.dump(self.topics, fdata, indent=4)

    def topics_by(self, username: str) -> list[forum.Topic]:
        return self.topics

    def create_post(
        self,
        content: str,
        topic_id: int | None = None,
        title: str | None = None,
        tags: list[str] | None = None,
        category_id: int | None = None,
    ) -> forum.Post:
        if topic_id is None:
            assert title, "Topic ID but no title in call"
            # Caller wants entirely new topic.
            topic = forum.Topic(
                post_stream={"posts": []},
                title=title,
                id=len(self.topics),
                posts_count=0,
                slug=title.replace(" ", "-"),
            )
            self.topics.append(topic)
            self._logger.warning(
                "Creating topic %s with title %r",
                topic["id"],
                topic["title"],
            )
        else:
            topic = self.topics[topic_id]

        post_id = 1000 + topic["id"] * 1000 + len(topic["post_stream"]["posts"])
        post = forum.Post(
            id=post_id,
            topic_slug=topic["slug"],
            topic_id=topic["id"],
            post_number=post_id,
            yours=True,
            raw=content,
            cooked=content,
            can_edit=True,
            reply_count=0,
        )
        topic["post_stream"]["posts"].append(post)
        topic["posts_count"] = len(topic["post_stream"]["posts"])

        self._logger.warning(
            "Creating post %s under topic %s with content %r",
            post["id"],
            post["topic_id"],
            post["raw"].strip()[:40],
        )

        self._persist()
        return post

    def update_post(self, post_id: int, content: str) -> None:
        post = self.post_by_id(post_id)
        assert post
        old = post["raw"].strip()
        new = content.strip()
        # Line-level unified diff (kept) for readability
        old_lines = old.splitlines(True)
        new_lines = new.splitlines(True)
        difference = "".join(difflib.unified_diff(old_lines, new_lines))
        url = f"{self.host.rstrip('/')}/t/{post['topic_slug']}/{post['topic_id']}/{post['post_number']}"
        if difference:
            self._logger.warning("Post %s SIMULATED updating => URL %s", post_id, url)
            self._logger.warning("  📝 DIFF:\n%s", difference)
            post["raw"] = content
            post["cooked"] = content
        else:
            self._logger.warning(
                "Post %s SIMULATED skipping update (no changes) => URL %s",
                post_id,
                url,
            )
        self._persist()

    def _get(self, api_url: str, page: int) -> forum.Topic:
        topic_id = int(api_url.split("/")[2].split(".")[0])
        if page > 1:
            raise RuntimeError("Mock DiscourseClient class does not support pages > 2")
        try:
            return [t for t in self.topics if t["id"] == topic_id][0]
        except IndexError:
            raise RuntimeError(f"Topic {topic_id} does not exist")

    def post_by_id(self, post_id: int) -> forum.Post | None:
        try:
            return [
                p
                for t in self.topics
                for p in t["post_stream"]["posts"]
                if p["id"] == post_id
            ][0]
        except IndexError:
            return None


class ForumClient(forum.ReleaseCandidateForumClient):
    def __init__(self, discourse_client: StubDiscourseClient):
        self.discourse_client = discourse_client  # type: ignore[assignment]
        self.nns_proposal_discussions_category_id = 0


class Github(object):
    pass


class ReleaseNotesClient(object):
    def __init__(self) -> None:
        if f := os.environ.get("RECONCILER_DRY_RUN_RELEASE_NOTES_STORAGE"):
            self.release_notes_folder = pathlib.Path(f)
            os.makedirs(self.release_notes_folder, exist_ok=True)
            self.release_notes_folder_cleanup = False
        else:
            self.release_notes_folder = pathlib.Path(
                tempfile.mkdtemp(prefix=f"reconciler-{self.__class__.__name__}-")
            )
            self.release_notes_folder_cleanup = True
        self._logger = LOGGER.getChild(self.__class__.__name__)

    def ensure(
        self,
        release_tag: str,
        release_commit: str,
        os_kind: OsKind,
        content: PreparedReleaseNotes,
    ) -> tuple[DocInfo, bool]:
        """
        Ensures the relase notes are stored in Google Docs (fake).

        Returns a DocInfo object, along with whether the doc changed.
        """
        t = self.release_notes_folder / (release_commit + os_kind)
        if t.exists():
            return {"alternateLink": str(t)}, False
        with open(t, "w") as f:
            f.write(f"{content}")
        self._logger.warning("Stored release notes in %s", t)
        return {"alternateLink": str(t)}, True

    def markdown_file(
        self, version: str, os_kind: OsKind
    ) -> PreparedReleaseNotes | None:
        try:
            with open((self.release_notes_folder / (version + os_kind)), "r") as f:
                return PreparedReleaseNotes(f.read())
        except FileNotFoundError:
            return None

    def __del__(self) -> None:
        if self.release_notes_folder_cleanup:
            shutil.rmtree(self.release_notes_folder)


class GitRepo(git_repo.GitRepo):
    def __init__(self, repo: str, **kwargs: typing.Any) -> None:
        super().__init__(repo, **kwargs)
        self._logger = LOGGER.getChild(self.__class__.__name__)

    def push_release_tags(self, release: Release) -> None:
        self._logger.warning(
            "Simulating push of tags associated with release %s", release
        )


class PublishNotesClient(object):
    def __init__(self) -> None:
        self._logger = LOGGER.getChild(self.__class__.__name__)

    def publish_if_ready(
        self,
        google_doc_markdownified: PreparedReleaseNotes | None,
        version: str,
        os_kind: OsKind,
    ) -> None:
        self._logger.warning(
            "Simulating that notes for release %s are ready to be published",
            version,
        )


class DRECli(dre_cli.DRECli):
    def __init__(
        self,
    ) -> None:
        super().__init__()
        self._logger = LOGGER.getChild(self.__class__.__name__)

    def propose_to_revise_elected_os_versions(
        self,
        changelog: str,
        version: str,
        os_kind: OsKind,
        forum_post_url: str,
        unelect_versions: list[str],
        package_checksum: str,
        package_urls: list[str],
        launch_measurements: typing.Optional[bytes],
        dry_run: bool = False,
    ) -> int:
        super().propose_to_revise_elected_os_versions(
            changelog,
            version,
            os_kind,
            forum_post_url,
            unelect_versions,
            package_checksum,
            package_urls,
            launch_measurements,
            dry_run=True,
        )
        # Now mock the proposal ID using an integer derived from the version.
        return crc32(version.encode("utf-8"))


class MockSlackAnnouncer(object):
    def __init__(self) -> None:
        self._logger = LOGGER.getChild(self.__class__.__name__)

    def announce_release(
        self,
        webhook: str,
        version_name: str,
        google_doc_url: str,
        os_kind: OsKind,
    ) -> None:
        self._logger.warning(
            "Simulating announcement of %s %s in slack", os_kind, version_name
        )


def oneoff_dre_place_proposal() -> None:
    changelog = "Fake changelog"
    dre = DRECli()
    measurements = {
        "guest_launch_measurements": [
            {
                "measurement": list(os.urandom(48)),
                "metadata": {
                    "kernel_cmdline": "some command line that is linked to this measaurement",
                },
            }
        ]
    }

    measurementbytes = json.dumps(measurements).encode()

    dre.propose_to_revise_elected_os_versions(
        changelog=changelog,
        version="0" * 40,
        os_kind=GUESTOS,
        forum_post_url="https://forum.dfinity.org/t/proposal-to-elect-new-release-rc-2024-03-27-23-01/29042/7",
        unelect_versions=[],
        package_checksum="0" * 40,
        package_urls=["https://doesntmatter.com/"],
        launch_measurements=measurementbytes,
    )


if __name__ == "__main__":
    # FIXME make formatter not output ANSI when stderr is not console
    oneoff_dre_place_proposal()
