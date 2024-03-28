def version_name(rc_name: str, name: str):
    return f"release-{rc_name.removeprefix("rc--")}-{name}"
