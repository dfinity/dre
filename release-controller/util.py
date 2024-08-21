import os


def version_name(rc_name: str, name: str):
    date = rc_name.removeprefix("rc--")
    return f"release-{date}-{name}"


def bazel_binary():
    if "BAZEL" in os.environ:
        return os.path.abspath(os.curdir) + "/release-controller/bazelisk"
    return "bazel"
