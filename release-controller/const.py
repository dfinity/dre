import typing

OsKind = typing.Literal["GuestOS"] | typing.Literal["HostOS"]

GUESTOS: typing.Literal["GuestOS"] = "GuestOS"
HOSTOS: typing.Literal["HostOS"] = "HostOS"

OS_KINDS: list[OsKind] = [GUESTOS, HOSTOS]

COMMIT_BELONGS: typing.Literal["True"] = "True"
COMMIT_DOES_NOT_BELONG: typing.Literal["False"] = "False"
COMMIT_COULD_NOT_BE_ANNOTATED: typing.Literal["Failed"] = "Failed"

CommitInclusionState = (
    typing.Literal["True"] | typing.Literal["False"] | typing.Literal["Failed"]
)
