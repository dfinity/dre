"""
The module fetches mainnet registry
"""

MAINNET_REGISTRY_BUILD = """
package(default_visibility = ["//visibility:public"])
exports_files(["mainnet_registry"])
"""

URL = "https://github.com/dfinity/dre/raw/ic-registry-mainnet/rs/ic-observability/multiservice-discovery/tests/test_data/mercury.gz"

def _mainnet_registry_impl(repository_ctx):
    repository_ctx.report_progress("Fetching " + repository_ctx.name)
    repository_ctx.download(url = URL, output = "mainnet_registry", sha256 = SHA256[platform], executable = False)
    repository_ctx.file("BUILD.bazel", MAINNET_REGISTRY_BUILD, executable = False)

_mainnet_registry = repository_rule(
    implementation = _mainnet_registry_impl,
    attrs = {},
)

def mainnet_registry(name):
    _mainnet_registry(name = name)
