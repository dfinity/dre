import typing

OsKind = typing.Literal["GuestOS"] | typing.Literal["HostOS"]

GUESTOS: typing.Literal["GuestOS"] = "GuestOS"
HOSTOS: typing.Literal["HostOS"] = "HostOS"

OS_KINDS: list[OsKind] = [GUESTOS, HOSTOS]


# It is safe to delete releases from this list once
# they disappear from file
# https://github.com/dfinity/dre/blob/main/release-index.yaml
IGNORED_RELEASES = [
    "rc--2024-03-06_23-01",
    "rc--2024-03-20_23-01",
    # From here on now we prevent the processing of releases that
    # would screw with the forum posts since their contents and
    # ordering in the threads have changed from this point on,
    # due to the addition of support for HostOS releases.
    "rc--2024-06-26_23-01",
    "rc--2024-07-03_23-01",
    "rc--2024-07-10_23-01",
    "rc--2024-07-25_21-03",
    "rc--2024-08-02_01-30",
    "rc--2024-08-08_07-48",
    "rc--2024-08-15_01-30",
    "rc--2024-08-21_15-36",
    "rc--2024-08-29_01-30",
    "rc--2024-09-06_01-30",
    "rc--2024-09-12_01-30",
    "rc--2024-09-19_01-31",
    "rc--2024-09-26_01-31",
    "rc--2024-10-03_01-30",
    "rc--2024-10-11_14-35",
    "rc--2024-10-17_03-07",
    "rc--2024-10-23_03-07",
    "rc--2024-10-31_03-09",
    "rc--2024-11-07_03-07",
    "rc--2024-11-14_03-07",
    "rc--2024-11-21_03-11",
    "rc--2024-11-28_03-15",
    "rc--2024-12-06_03-16",
    "rc--2025-01-03_03-07",
    "rc--2025-01-09_03-19",
    "rc--2025-01-16_16-18",
    "rc--2025-01-23_03-04",
    "rc--2025-01-30_03-03",
    "rc--2025-02-06_12-26",
    "rc--2025-02-13_03-06",
    "rc--2025-02-20_10-16",
    "rc--2025-02-27_03-09",
    "rc--2025-03-06_03-10",
    "rc--2025-03-14_03-10",
    "rc--2025-03-20_03-11",
    "rc--2025-03-27_03-14",
    "rc--2025-04-03_03-15",
    "rc--2025-04-10_03-16",
    "rc--2025-04-11_13-20",
]
COMMIT_BELONGS: typing.Literal["True"] = "True"
COMMIT_DOES_NOT_BELONG: typing.Literal["False"] = "False"
COMMIT_COULD_NOT_BE_ANNOTATED: typing.Literal["Failed"] = "Failed"

CommitInclusionState = (
    typing.Literal["True"] | typing.Literal["False"] | typing.Literal["Failed"]
)
