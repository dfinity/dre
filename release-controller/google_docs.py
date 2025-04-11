import pathlib
import tempfile
import typing

import mammoth
import markdown
from markdownify import markdownify
from pydrive2.auth import GoogleAuth
from pydrive2.drive import GoogleDrive

from const import OsKind, GUESTOS
from git_repo import GitRepo
from release_notes import PreparedReleaseNotes, LocalCommitChangeDeterminator

md = markdown.Markdown(
    extensions=["pymdownx.tilde", "pymdownx.details"],
)


pathlib.Path(__file__).parent.resolve()


class DocInfo(typing.TypedDict):
    alternateLink: str


class ReleaseNotesClientProtocol(typing.Protocol):
    def ensure(
        self,
        release_tag: str,
        release_commit: str,
        os_kind: OsKind,
        content: PreparedReleaseNotes,
    ) -> DocInfo: ...

    def markdown_file(
        self, version: str, os_kind: OsKind
    ) -> PreparedReleaseNotes | None: ...


class ReleaseNotesClient:
    """Client for managing release notes in Google Drive."""

    def __init__(
        self,
        credentials_file: pathlib.Path,
        release_notes_folder: str = "1y-nuH29Gd5Err3pazYH6-LzcDShcOIFf",
    ) -> None:
        """Create a new ReleaseNotesClient."""
        settings = {
            "client_config_backend": "service",
            "service_config": {
                "client_json_file_path": credentials_file,
            },
        }
        self.release_notes_folder = release_notes_folder

        gauth = GoogleAuth(settings=settings)  # type: ignore[no-untyped-call]
        gauth.ServiceAuth()
        self.drive = GoogleDrive(gauth)  # type: ignore[no-untyped-call]

    def ensure(
        self,
        release_tag: str,
        release_commit: str,
        os_kind: OsKind,
        content: PreparedReleaseNotes,
    ) -> DocInfo:
        """
        Ensure that a release notes document exists for the given version.

        No changes are effected if the document mapped to this release commit
        already exists.
        """
        existing_file = self._file(release_commit, os_kind)
        if existing_file:
            return typing.cast(DocInfo, existing_file)
        htmldoc = md.convert(content)
        gdoc = self.drive.CreateFile(  # type: ignore[no-untyped-call]
            {
                "title": f"{os_kind} Release Notes - {release_tag} ({release_commit})",
                "mimeType": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                "parents": [
                    {"kind": "drive#fileLink", "id": self.release_notes_folder}
                ],
            }
        )
        gdoc.SetContentString(htmldoc)
        gdoc.Upload()
        return typing.cast(DocInfo, gdoc)

    def _file(self, version: str, os_kind: OsKind) -> DocInfo | None:
        """Get the file for the given version."""
        release_notes = self.drive.ListFile(  # type: ignore[no-untyped-call]
            {"q": "'{}' in parents".format(self.release_notes_folder)}
        ).GetList()
        for file in release_notes:
            if version in file["title"] and os_kind in file["title"]:
                return typing.cast(DocInfo, file)
        return None

    def markdown_file(
        self, version: str, os_kind: OsKind
    ) -> PreparedReleaseNotes | None:
        """Get the markdown content of the release notes for the given version."""
        f = self._file(version, os_kind)
        if not f:
            return None
        with tempfile.TemporaryDirectory() as d:
            release_docx = pathlib.Path(d) / "release.docx"
            f.GetContentFile(release_docx)  # type: ignore[attr-defined]
            return google_doc_to_markdown(release_docx)


def google_doc_to_markdown(release_docx: pathlib.Path) -> PreparedReleaseNotes:
    # google docs will convert the document to docx format first time it's saved
    # before that, it should be in html
    try:
        with open(
            release_docx, "tr", encoding="utf8"
        ) as f:  # try open file in text mode
            release_html = f.read()
    except Exception:  # if fail then file is non-text (binary)  # noqa: E722  # pylint: disable=bare-except
        release_html = mammoth.convert_to_html(open(release_docx, "rb")).value  # type: ignore[no-untyped-call]

    release_md = markdownify(release_html)  # type: ignore[no-untyped-call]
    return typing.cast(PreparedReleaseNotes, release_md)


def main() -> None:
    from release_notes import prepare_release_notes, OrdinaryReleaseNotesRequest

    client = ReleaseNotesClient(
        credentials_file=pathlib.Path(__file__).parent.resolve() / "credentials.json",
        release_notes_folder="1zOPwbYdOREhhLv-spRIRRMaFaAQlOVvF",
    )
    version = "3d0b3f10417fc6708e8b5d844a0bac5e86f3e17d"

    request = OrdinaryReleaseNotesRequest(
        "release-2024-08-02_01-30-base",
        version,
        "release-2024-07-25_21-03-base",
        "2c0b76cfc7e596d5c4304cff5222a2619294c8c1",
        GUESTOS,
    )
    ic_repo = GitRepo("https://github.com/dfinity/ic.git", main_branch="master")
    content = prepare_release_notes(
        request, ic_repo, LocalCommitChangeDeterminator(ic_repo).commit_changes_artifact
    )
    gdoc = client.ensure(
        release_tag="release-2024-08-02_01-30-base",
        release_commit=version,
        content=content,
        os_kind=GUESTOS,
    )
    print(client.markdown_file(version, GUESTOS))
    print(gdoc["alternateLink"])


if __name__ == "__main__":
    main()
