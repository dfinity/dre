import os
import tempfile
from pydrive2.auth import GoogleAuth
from pydrive2.drive import GoogleDrive
from pydrive2.files import GoogleDriveFile
from markdownify import markdownify
from release_notes import release_notes
import markdown
import slack

md = markdown.Markdown(extensions=["pymdownx.tilde"])

import pathlib

pathlib.Path(__file__).parent.resolve()

release_notes_folder = "1y-nuH29Gd5Err3pazYH6-LzcDShcOIFf"


class ReleaseNotesClient:
    def __init__(self, credentials_file: pathlib.Path):
        settings = {
            "client_config_backend": "service",
            "service_config": {
                "client_json_file_path": credentials_file,
            },
        }

        gauth = GoogleAuth(settings=settings)
        gauth.ServiceAuth()
        self.drive = GoogleDrive(gauth)

    def ensure(self, version_name: str, version: str, content: str):
        existing_file = self.file(version)
        if existing_file:
            return existing_file
        htmldoc = md.convert(content)
        gdoc = self.drive.CreateFile(
            {
                "title": f"Release Notes - {version_name} ({version})",
                "mimeType": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                "parents": [{"kind": "drive#fileLink", "id": release_notes_folder}],
            }
        )
        gdoc.SetContentString(htmldoc)
        gdoc.Upload()
        slack.announce_release(
            os.environ["SLACK_WEBHOOK_URL"],
            "release-2024-03-06_23-01+p2p",
            gdoc["alternateLink"],
            False,
        )
        return gdoc

    def file(self, version: str):
        release_notes = self.drive.ListFile({"q": "'{}' in parents".format(release_notes_folder)}).GetList()
        for file in release_notes:
            if version in file["title"]:
                return file
        return None

    def markdown_file(self, version):
        f = self.file(version)
        if not f:
            return None
        with tempfile.TemporaryDirectory() as d:
            release_html = pathlib.Path(d) / "release.html"
            f.GetContentFile(release_html, mimetype="text/html")
            release_md = markdownify(open(release_html, "r").read())
            # TODO: parse markdown to check formatting is correct
            return release_md

    def archive_inactive(self, active_versions: list[str]):
        pass


def main():
    client = ReleaseNotesClient(credentials_file=pathlib.Path(__file__).parent.resolve() / "credentials.json")
    release = "rc--2024-02-21_23-01"
    version = "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f"
    notes = release_notes("8d4b6898d878fa3db4028b316b78b469ed29f293", version, release)
    print(notes)
    gdoc = client.ensure(version_name=release, version=version, content=notes)
    print(gdoc["alternateLink"])


if __name__ == "__main__":
    main()
