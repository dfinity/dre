import os


def version_name(rc_name: str, name: str):
    date = rc_name.removeprefix("rc--")
    return f"release-{date}-{name}"


def resolve_binary(name: str):
    """
    Resolve the binary path for the given binary name.
    Try to locate the binary in expected location if it was packaged in an OCI image.
    """
    binary_local = os.path.join(os.path.dirname(__file__), name)
    if os.path.exists(binary_local):
        return binary_local
    binary_local = os.path.join("/rs/cli", name)
    if name == "dre" and os.path.exists(binary_local):
        return binary_local
    return name
