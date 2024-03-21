import mimetypes
import os
import tempfile
import time
import mammoth
from pydrive2.auth import GoogleAuth
from pydrive2.drive import GoogleDrive
from pydrive2.files import GoogleDriveFile
from markdownify import markdownify
from release_notes import release_notes
import markdown
import slack

md = markdown.Markdown(
    extensions=["pymdownx.tilde"],
)

import pathlib

pathlib.Path(__file__).parent.resolve()


class ReleaseNotesClient:
    def __init__(self, credentials_file: pathlib.Path, release_notes_folder="1y-nuH29Gd5Err3pazYH6-LzcDShcOIFf"):
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

    def ensure(self, version_name: str, version: str, content: str, tag_teams_on_create: bool):
        existing_file = self.file(version)
        if existing_file:
            return existing_file
        htmldoc = md.convert(content)
        gdoc = self.drive.CreateFile(
            {
                "title": f"Release Notes - {version_name} ({version})",
                "mimeType": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                "parents": [{"kind": "drive#fileLink", "id": self.release_notes_folder}],
            }
        )
        gdoc.SetContentString(htmldoc)
        gdoc.Upload()
        if "SLACK_WEBHOOK_URL" in os.environ:
            slack.announce_release(
                slack_url=os.environ["SLACK_WEBHOOK_URL"],
                version_name=version_name,
                google_doc_url=gdoc["alternateLink"],
                tag_all_teams=tag_teams_on_create,
            )
        return gdoc

    def file(self, version: str):
        release_notes = self.drive.ListFile({"q": "'{}' in parents".format(self.release_notes_folder)}).GetList()
        for file in release_notes:
            if version in file["title"]:
                return file
        return None

    def markdown_file(self, version):
        f = self.file(version)
        if not f:
            return None
        with tempfile.TemporaryDirectory() as d:
            release_docx = pathlib.Path(d) / "release.docx"
            f.GetContentFile(release_docx)
            # google docs will convert the document to docx format first time it's saved
            # before that, it should be in html
            try:
                with open(release_docx, "tr") as f:  # try open file in text mode
                    release_html = f.read()
            except:  # if fail then file is non-text (binary)
                release_html = mammoth.convert_to_html(open(release_docx, "rb")).value

            release_md = markdownify(release_html)
            return release_md

    def archive_inactive(self, active_versions: list[str]):
        pass


def main():
    client = ReleaseNotesClient(
        credentials_file=pathlib.Path(__file__).parent.resolve() / "credentials.json",
        release_notes_folder="1zOPwbYdOREhhLv-spRIRRMaFaAQlOVvF",
    )
    release = "rc--2024-02-21_23-01"
    version = "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f"
    notes = release_notes("8d4b6898d878fa3db4028b316b78b469ed29f293", version, release)
    print(notes)
    gdoc = client.ensure(version_name=release, version=version, content=notes, tag_teams_on_create=False)
    print(client.markdown_file(version))
    print(gdoc["alternateLink"])


if __name__ == "__main__":
    main()
