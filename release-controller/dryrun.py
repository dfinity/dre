import logging
import os
import shutil
import sys
import tempfile
import pathlib
import typing
import git_repo
import dre_cli
import json

from release_notes import PreparedReleaseNotes
from release_index import Release

me = os.path.join(os.path.dirname(__file__))
if me not in sys.path:
    sys.path.append(me)


import forum  # noqa: E402
import pydiscourse  # noqa: E402


# FIXME: types in callsites for the bottom classes (in particular reconciler.py)
# should all be made protocols.

LOGGER = logging.getLogger(__name__)


class FakePost(typing.TypedDict):
    id: int
    topic_id: int
    posts_count: int
    yours: bool
    topic_slug: str
    raw: str
    content: str
    can_edit: bool
    title: str
    post_number: int


class FakeTopic(FakePost):
    replies: list[FakePost]


class PostsList(typing.TypedDict):
    posts: list[FakePost]


class PostStream(typing.TypedDict):
    post_stream: PostsList


class DiscourseClient(object):
    def __init__(self) -> None:
        self.topics: list[FakeTopic] = []
        self.api_username = "doesntmatter"
        self.host = "fakediscourse.com.internal"
        self._logger = LOGGER.getChild(self.__class__.__name__)

        if f := os.environ.get("DRY_RUN_FORUM_STORAGE"):
            self.forum_storage: pathlib.PosixPath | None = pathlib.PosixPath(f)
            os.makedirs(self.forum_storage, exist_ok=True)
            try:
                with open(self.forum_storage / "mock-posts.json", "rb") as fdata:
                    self.topics = json.load(fdata)
            except FileNotFoundError:
                pass
        else:
            self.forum_storage = None

    def _persist(self):
        for topic in self.topics:
            print(f"* Topic {topic['id']} titled {topic['title']}")
            print(f"  Content {topic['content'].splitlines()[0].strip()}")
            for reply in topic["replies"]:
                print(f"  * Reply {reply['id']} {reply['topic_id']}")
                print(f"    Content {reply['content'].splitlines()[0].strip()}")
        if self.forum_storage:
            with open(self.forum_storage / "mock-posts.json", "w") as fdata:
                json.dump(self.topics, fdata, indent=4)

    def topics_by(self, username: str) -> list[FakeTopic]:
        return self.topics

    def create_post(self, **kwargs: typing.Any) -> dict[str, typing.Any]:
        post = typing.cast(FakeTopic, kwargs)
        post["id"] = len(self.topics)
        self._logger.warning(
            "Creating post %s with title %r and content %r",
            post["id"],
            post.get("title", "(no title)"),
            post["content"],
        )
        post["posts_count"] = 1
        post["yours"] = True
        post["post_number"] = post["id"]
        post["topic_slug"] = "slug-of-topic-" + str(post["id"])
        post["raw"] = post["content"]
        post["can_edit"] = True
        if "topic_id" in post:
            topic = [p for p in self.topics if p["id"] == post["topic_id"]][0]
            topic["replies"].append(post)
        else:
            post["replies"] = []
            self.topics.append(post)
            post["topic_id"] = post["id"]
        self._persist()
        return kwargs

    def update_post(self, post_id: int, content: str):
        post = self.post_by_id(post_id)
        assert post
        if content != post["content"]:
            self._logger.warning(
                "Updating post %s titled %r",
                post["id"],
                post.get("title", "(no title)"),
            )
            self._logger.warning("* Old content: %r", post["content"][:40])
            self._logger.warning("* New content: %r", content[:40])
            post["content"] = content
            post["raw"] = content
        self._persist()

    def _get(self, api_url: str, page: int) -> PostStream | None:
        topic_id = int(api_url.split("/")[2].split(".")[0])
        if page < 2:
            return {
                "post_stream": {
                    "posts": [t for t in self.topics if t["topic_id"] == topic_id]
                    + [
                        r
                        for t in self.topics
                        for r in t["replies"]
                        if r["topic_id"] == topic_id
                    ]
                }
            }
        return None

    def post_by_id(self, post_id: int) -> FakePost | None:
        try:
            return [t for t in self.topics if t["id"] == post_id][0]
        except IndexError:
            try:
                return [
                    r for t in self.topics for r in t["replies"] if r["id"] == post_id
                ][0]
            except IndexError:
                return None


class ForumClient(forum.ReleaseCandidateForumClient):
    def __init__(self, discourse_client: pydiscourse.DiscourseClient):
        self.discourse_client = discourse_client
        self.nns_proposal_discussions_category_id = 0


class Github(object):
    pass


class ReleaseNotesClient(object):
    def __init__(self):
        if f := os.environ.get("DRY_RUN_RELEASE_NOTES_STORAGE"):
            self.release_notes_folder = pathlib.PosixPath(f)
            os.makedirs(self.release_notes_folder, exist_ok=True)
            self.release_notes_folder_cleanup = False
        else:
            self.release_notes_folder = pathlib.PosixPath(tempfile.mkdtemp())
            self.release_notes_folder_cleanup = True
        self._logger = LOGGER.getChild(self.__class__.__name__)

    def has_release_notes(self, release_commit: str) -> bool:
        return (self.release_notes_folder / release_commit).exists()

    def ensure(
        self, release_tag: str, release_commit: str, content: PreparedReleaseNotes
    ) -> typing.Any:
        t = self.release_notes_folder / release_commit
        if t.exists():
            return t
        with open(t, "w") as f:
            f.write(f"{content}")
        self._logger.warning("Stored release notes in %s", t)
        return t

    def markdown_file(self, version: str) -> PreparedReleaseNotes:
        with open((self.release_notes_folder / version), "r") as f:
            return PreparedReleaseNotes(f.read())

    def __del__(self):
        if self.release_notes_folder_cleanup:
            shutil.rmtree(self.release_notes_folder)


class GitRepo(git_repo.GitRepo):
    def __init__(self, repo: str, **kwargs):
        super().__init__(repo, **kwargs)
        self._logger = LOGGER.getChild(self.__class__.__name__)

    def add_note(self, namespace: str, object: str, content: str) -> None:
        raise NotImplementedError()

    def push_release_tags(self, release: Release) -> None:
        self._logger.warning(
            "Simulating push of tags associated with release %s", release
        )


class PublishNotesClient(object):
    def __init__(self):
        self._logger = LOGGER.getChild(self.__class__.__name__)

    def publish_if_ready(
        self, google_doc_markdownified: PreparedReleaseNotes, version: str
    ) -> None:
        self._logger.warning(
            "Simulating that notes for release %s are not ready", version
        )


class DRECli(dre_cli.DRECli):
    def __init__(self):
        super().__init__()
        self._logger = LOGGER.getChild(self.__class__.__name__)

    def place_proposal(
        self,
        changelog,
        version: str,
        forum_post_url: str,
        unelect_versions: list[str],
        package_checksum: str,
        package_urls: list[str],
        dry_run=False,
    ):
        super().place_proposal(
            changelog,
            version,
            forum_post_url,
            unelect_versions,
            package_checksum,
            package_urls,
            dry_run=True,
        )


class MockSlackAnnouncer(object):
    def __init__(self):
        self._logger = LOGGER.getChild(self.__class__.__name__)

    def announce_release(
        self, webhook: str, version_name: str, google_doc_url: str, tag_all_teams: bool
    ) -> None:
        self._logger.warning("Simulating announcement of %s in slack", version_name)
