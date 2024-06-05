def version_name(rc_name: str, name: str):
    date = rc_name.removeprefix("rc--")
    return f"release-{date}-{name}"
