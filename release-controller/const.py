import typing

OsKind = typing.Literal["GuestOS"] | typing.Literal["HostOS"]

GUESTOS: typing.Literal["GuestOS"] = "GuestOS"
HOSTOS: typing.Literal["HostOS"] = "HostOS"

OS_KINDS: list[OsKind] = [GUESTOS, HOSTOS]
