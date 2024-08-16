import os
import pathlib
import tempfile
import typing

import mammoth
import markdown
import slack_announce
from markdownify import markdownify
from pydrive2.auth import GoogleAuth
from pydrive2.drive import GoogleDrive
from release_notes import prepare_release_notes

md = markdown.Markdown(
    extensions=["pymdownx.tilde", "pymdownx.details"],
)


pathlib.Path(__file__).parent.resolve()


class ReleaseNotesClient:
    """Client for managing release notes in Google Drive."""

    def __init__(self, credentials_file: pathlib.Path, release_notes_folder="1y-nuH29Gd5Err3pazYH6-LzcDShcOIFf"):
        """Create a new ReleaseNotesClient."""
        settings = {
            "client_config_backend": "service",
            "service_config": {
                "client_json_file_path": credentials_file,
            },
        }
        self.release_notes_folder = release_notes_folder

        gauth = GoogleAuth(settings=settings)
        gauth.ServiceAuth()
        self.drive = GoogleDrive(gauth)

    def ensure(self, base_release_tag, base_release_commit, release_tag, release_commit, tag_teams_on_create: bool):
        """Ensure that a release notes document exists for the given version."""
        existing_file = self.file(release_commit)
        if existing_file:
            return existing_file
        content = prepare_release_notes(base_release_tag, base_release_commit, release_tag, release_commit)
        htmldoc = md.convert(content)
        gdoc = self.drive.CreateFile(
            {
                "title": f"Release Notes - {release_tag} ({release_commit})",
                "mimeType": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                "parents": [{"kind": "drive#fileLink", "id": self.release_notes_folder}],
            }
        )
        gdoc.SetContentString(htmldoc)
        gdoc.Upload()
        if "SLACK_WEBHOOK_URL" in os.environ:
            slack_announce.announce_release(
                slack_url=os.environ["SLACK_WEBHOOK_URL"],
                version_name=release_tag,
                google_doc_url=gdoc["alternateLink"],
                tag_all_teams=tag_teams_on_create,
            )
        return gdoc

    def file(self, version: str):
        """Get the file for the given version."""
        release_notes = self.drive.ListFile({"q": "'{}' in parents".format(self.release_notes_folder)}).GetList()
        for file in release_notes:
            if version in file["title"]:
                return file
        return None

    def markdown_file(self, version):
        """Get the markdown content of the release notes for the given version."""
        f = self.file(version)
        if not f:
            return None
        with tempfile.TemporaryDirectory() as d:
            release_docx = pathlib.Path(d) / "release.docx"
            f.GetContentFile(release_docx)
            # google docs will convert the document to docx format first time it's saved
            # before that, it should be in html
            try:
                with open(release_docx, "tr", encoding="utf8") as f:  # try open file in text mode
                    release_html = f.read()
            except:  # if fail then file is non-text (binary)  # noqa: E722  # pylint: disable=bare-except
                release_html = mammoth.convert_to_html(open(release_docx, "rb")).value

            release_md = markdownify(release_html)
            return release_md


def main():
    client = ReleaseNotesClient(
        credentials_file=pathlib.Path(__file__).parent.resolve() / "credentials.json",
        release_notes_folder="1zOPwbYdOREhhLv-spRIRRMaFaAQlOVvF",
    )
    version = "3d0b3f10417fc6708e8b5d844a0bac5e86f3e17d"
    gdoc = client.ensure(
        release_tag="release-2024-08-02_01-30-base",
        release_commit=version,
        base_release_commit="2c0b76cfc7e596d5c4304cff5222a2619294c8c1",
        base_release_tag="release-2024-07-25_21-03-base",
        tag_teams_on_create=False,
    )
    print(client.markdown_file(version))
    print(gdoc["alternateLink"])


if __name__ == "__main__":
    main()
