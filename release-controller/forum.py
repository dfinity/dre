import difflib
import logging
import os
from typing import cast, Callable, TypedDict, Protocol, ParamSpec

from dotenv import load_dotenv
from pydiscourse import DiscourseClient
from cached_discourse import CachedDiscourse
from release_index import Release, Version
from util import version_name
import reconciler_state
from const import OsKind, GUESTOS, HOSTOS

LOGGER = logging.getLogger(__name__)

P = ParamSpec("P")


def trim_end_whitespace(f: Callable[P, str]) -> Callable[P, str]:
    """Make any function that returns string strip the end whitespace of its return value."""

    def inner(*args: P.args, **kwargs: P.kwargs) -> str:
        return f(*args, **kwargs).rstrip()

    return inner


@trim_end_whitespace
def _post_template(
    changelog: str | None,
    version_name: str,
    os_kind: OsKind,
    proposal: reconciler_state.NoProposal
    | reconciler_state.DREMalfunction
    | reconciler_state.SubmittedProposal,
) -> str:
    """
    Produces a release post for Discourse based on the inputs.

    To developers, an **important** note:

    Always make sure that the value returned by this post does not have any
    ending carriage returns.  Otherwise release controller always attempts
    to update posts spuriously on startup, since Discourse eats up carriage
    returns at the end of the post.

    Thus we have a decorator to ensure this.
    """
    if isinstance(proposal, reconciler_state.NoProposal):
        ret = f"We're preparing [a new IC release](https://github.com/dfinity/ic/tree/{version_name})."
        if changelog:
            ret += f"\n\nThe following is a **draft** of the list of changes since the last {os_kind} release:\n\n{changelog}"
        return ret

    elif isinstance(proposal, reconciler_state.DREMalfunction):
        return (
            f"A proposal to adopt [a new {os_kind} release](https://github.com/dfinity/ic/tree/{version_name}) has been prepared,"
            " but a temporary hiccup has taken place, preventing the proposal ID from being obtained."
            " The proposal ID and the changelog will be announced soon."
        )

    return f"""\
Hello there!

We are happy to announce that voting is now open for [a new {os_kind} release](https://github.com/dfinity/ic/tree/{version_name}).
The NNS proposal is here: [IC NNS Proposal {proposal.proposal_id}](https://dashboard.internetcomputer.org/proposal/{proposal.proposal_id}).

Here is a summary of the changes since the last {os_kind} release:

{changelog}"""


class ReleaseCandidateForumPost:
    """A post in a release candidate forum topic."""

    def __init__(
        self,
        version_name: str,
        changelog: str | None,
        proposal: reconciler_state.NoProposal
        | reconciler_state.DREMalfunction
        | reconciler_state.SubmittedProposal,
        os_kind: OsKind,
        security_fix: bool = False,
    ):
        """Create a new post."""
        self.version_name = version_name
        self.changelog = changelog
        self.proposal = proposal
        self.os_kind: OsKind = os_kind
        self.security_fix = security_fix


SummaryRetriever = Callable[[str, OsKind, bool], str | None]


class Post(TypedDict):
    id: int
    topic_id: int
    topic_slug: str
    reply_count: int
    post_number: int
    yours: bool
    raw: str
    cooked: str
    can_edit: bool


class PostStream(TypedDict):
    posts: list[Post]


class Topic(TypedDict):
    post_stream: PostStream
    title: str
    id: int
    posts_count: int
    slug: str


class ReleaseCandidateForumTopic:
    """A topic in the governance category for a release candidate."""

    def __init__(
        self,
        release: Release,
        client: DiscourseClient,
        nns_proposal_discussions_category_id: int,
    ):
        """Create a new topic."""
        self._logger = LOGGER.getChild(self.__class__.__name__)
        self.posts_count = 1
        self.release = release
        self.client = CachedDiscourse(client)
        self.nns_proposal_discussions_category_id = nns_proposal_discussions_category_id
        topic = next(
            (
                t
                for t in self.client.topics_by(self.client.api_username)
                if self.release.rc_name in t.get("title", "")
            ),
            None,
        )
        if topic:
            self.topic_id = topic["id"]
            self.posts_count = topic["posts_count"]
        else:
            post = client.create_post(  # type: ignore[no-untyped-call]
                category_id=nns_proposal_discussions_category_id,
                content="The proposal for the next release will be announced soon.",
                tags=["IC-OS-election", "release"],
                title=f"Proposal to elect new release {self.release.rc_name}",
            )
            if post:
                self.topic_id = post["topic_id"]
            else:
                raise RuntimeError("post not created")

    def created_posts(self) -> list[Post]:
        """Return a list of posts created by the current user."""
        results = [
            post
            for page in range((self.posts_count - 1) // 20 + 1)
            for post in self.client.topic_page(self.topic_id, page)["post_stream"][
                "posts"
            ]
            if post["yours"]
        ]
        if not results:
            raise RuntimeError("failed to list topic posts")
        return results

    def update(
        self,
        summary_retriever: SummaryRetriever,
        proposal_id_retriever: reconciler_state.ProposalRetriever,
    ) -> None:
        """Update the topic with the latest release information."""
        posts: list[ReleaseCandidateForumPost] = [
            poast
            for os_kind, vers in cast(
                list[tuple[OsKind, list[Version]]],
                [
                    (GUESTOS, self.release.versions[:1]),  # base release guestos
                    (HOSTOS, self.release.versions[:1]),  # base release hostos
                    (
                        GUESTOS,
                        self.release.versions[1:],
                    ),  # all other feature releases, not supported for hostos
                ],
            )
            for v in vers
            for poast in [
                ReleaseCandidateForumPost(
                    version_name=version_name(self.release.rc_name, v.name),
                    changelog=summary_retriever(v.version, os_kind, v.security_fix),
                    proposal=proposal_id_retriever(v.version, os_kind),
                    security_fix=v.security_fix,
                    os_kind=os_kind,
                )
            ]
        ]

        created_posts = self.created_posts()
        for i, p in enumerate(posts):
            if i < len(created_posts):
                post_id = created_posts[i]["id"]
                content_expected = _post_template(
                    version_name=p.version_name,
                    changelog=p.changelog,
                    proposal=p.proposal,
                    os_kind=p.os_kind,
                )
                # Reuse the already-fetched post from the cache
                post = created_posts[i]
                old = post["raw"].rstrip()
                new = content_expected.rstrip()
                if old == new:
                    # log the complete URL of the post
                    self._logger.debug("Post up to date: %s.", self.post_to_url(post))
                    continue
                else:
                    self._logger.info(
                        "Post %s differs (URL: %s):", post_id, self.post_to_url(post)
                    )
                    difference = difflib.unified_diff(
                        old.splitlines(True), new.splitlines(True)
                    )
                    self._logger.info("  📝 DIFF:\n%s", "".join(difference))
                    if post["can_edit"]:
                        self._logger.info(
                            "Post %s updating => URL %s",
                            post_id,
                            self.post_to_url(post),
                        )
                        self.client.update_post(
                            post_id=post_id, content=content_expected
                        )
                        # Ensure there is ALWAYS a log when an update occurs
                        self._logger.info(
                            "Post %s updated => URL %s",
                            post_id,
                            self.post_to_url(post),
                        )
                        self.client.invalidate_topic(self.topic_id)
                    else:
                        self._logger.warning(
                            "Post %s NOT editable. Skipping update => URL %s",
                            post_id,
                            self.post_to_url(post),
                        )
            else:
                self._logger.debug("Creating new post.")
                self.client.create_post(
                    topic_id=self.topic_id,
                    content=_post_template(
                        version_name=p.version_name,
                        changelog=p.changelog,
                        proposal=p.proposal,
                        os_kind=p.os_kind,
                    ),
                )

    def post_url(self, version: str) -> str:
        """Return the URL of the post for the given version."""
        post_index = [
            i for i, v in enumerate(self.release.versions) if v.version == version
        ][0]
        post = self.client.post_by_id(post_id=self.created_posts()[post_index]["id"])
        if not post:
            raise RuntimeError("failed to find post")
        return self.post_to_url(post)

    def post_to_url(self, post: Post) -> str:
        """Return the complete URL of the given post."""
        host = self.client.host.removesuffix("/")
        return f"{host}/t/{post['topic_slug']}/{post['topic_id']}/{post['post_number']}"

    def add_version(self, content: str) -> None:
        """Add a new version to the topic."""
        self.client.create_post(
            topic_id=self.topic_id,
            content=content,
        )


class ForumClientProtocol(Protocol):
    def get_or_create(self, release: Release) -> ReleaseCandidateForumTopic: ...


class ReleaseCandidateForumClient:
    """A client for interacting with release candidate forum topics."""

    def __init__(self, discourse_client: DiscourseClient):
        """Create a new client."""
        self.discourse_client = discourse_client
        existing_categories = [
            c
            for c in self.discourse_client.categories(include_subcategories="true")  # type: ignore[no-untyped-call]
            if c["name"] == "NNS proposal discussions"
        ]
        if existing_categories:
            self.nns_proposal_discussions_category_id: int = existing_categories[0][
                "id"
            ]
        else:
            self.nns_proposal_discussions_category_id = self.discourse_client.category(  # type: ignore[no-untyped-call]
                76
            )[
                "category"
            ][
                "id"
            ]  # hardcoded category id, seems like "include_subcategories" is not working

    def get_or_create(self, release: Release) -> ReleaseCandidateForumTopic:
        """Get or create a forum topic for the given release."""
        return ReleaseCandidateForumTopic(
            release=release,
            client=self.discourse_client,
            nns_proposal_discussions_category_id=self.nns_proposal_discussions_category_id,
        )


def main() -> None:
    load_dotenv()

    discourse_client = DiscourseClient(  # type: ignore[no-untyped-call]
        host=os.environ["DISCOURSE_URL"],
        api_username=os.environ["DISCOURSE_USER"],
        api_key=os.environ["DISCOURSE_KEY"],
    )
    #     index = parse_yaml_raw_as(
    #         Model,
    #         """
    # rollout:
    #   stages: []

    # releases:
    #   - rc_name: rc--2024-03-13_23-05
    #     versions:
    #       - version: 2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f
    #         name: default
    #       - version: 31e9076fb99dfc36eb27fb3a2edc68885e6163ac
    #         name: feat
    #       - version: db583db46f0894d35bcbcfdea452d93abdadd8a6
    #         name: feat-hotfix1
    # """,
    #     )
    _forum_client = ReleaseCandidateForumClient(
        discourse_client,
    )


#     topic = forum_client.get_or_create(index.root.releases[0])
#     topic.update(lambda _, _: None, lambda _: None)

# print(topic.post_url(version="31e9076fb99dfc36eb27fb3a2edc68885e6163ac"))


if __name__ == "__main__":
    main()
