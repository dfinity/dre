import json
import subprocess
import typing
from util import resolve_binary
import os


class Auth(typing.TypedDict):
    key_path: str
    neuron_id: str


class DRECli:
    def __init__(self, auth: typing.Optional[Auth] = None):
        self.env = os.environ.copy()
        if auth:
            self.auth = [
                "--private-key-pem",
                auth["key_path"],
                "--neuron-id",
                auth["neuron_id"],
            ]
        else:
            self.auth = []
        self.cli = resolve_binary("dre")

    def run(self, *args):
        """Run the dre CLI."""
        return subprocess.check_output([self.cli, *self.auth, *args], env=self.env)

    def get_blessed_versions(self):
        """Query the blessed versions."""
        return json.loads(
            subprocess.check_output(
                [self.cli, "get", "blessed-replica-versions", "--json"], env=self.env
            )
        )
