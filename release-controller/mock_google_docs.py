from google_docs import ReleaseNotesClient


class ReleaseNotesClientMock(ReleaseNotesClient):
    def __init__(self):
        self.notes = {}

    def ensure(self, version_name: str, version: str, content: str):
        if version in self.notes:
            return
        self.notes[version] = content

    def file(self, version: str):
        raise RuntimeError("not implemented")

    def markdown_file(self, version: str):
        return self.notes.get(version, None)
