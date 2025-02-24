# Docs at https://github.com/theoremlp/rules_mypy .

load("@rules_mypy//mypy:mypy.bzl", "mypy")
load("@pip_types//:types.bzl", "types")

mypy_aspect = mypy(
    mypy_ini = "@@//:mypy.ini",
    opt_in_tags = ["typecheck"],
    types = types,
)
