import os


def version_name(rc_name: str, name: str):
    date = rc_name.removeprefix("rc--")
    return f"release-{date}-{name}"


def bazel_binary():
    bazel_binary = "bazel"
    bazel_binary_local = os.path.abspath(os.curdir) + "/release-controller/bazelisk"
    if os.path.exists(bazel_binary_local):
        bazel_binary = bazel_binary_local
    return bazel_binary
