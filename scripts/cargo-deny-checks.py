#!/usr/bin/env python3
"""Check Cargo.lock for a) disallowed crates or b) multiple versions of crates required to have a single version."""

import sys

try:
    import tomllib  # Available in Python 3.11 and later
except ModuleNotFoundError:
    import tomli as tomllib  # Third-party package for earlier versions

DISALLOWED_CRATES = {"openssl", "openssl-sys"}
REQUIRE_SINGLE_VERSION = {"opentelemetry"}


def main():
    try:
        with open("Cargo.lock", "rb") as f:
            data = tomllib.load(f)
    except FileNotFoundError:
        print("Cargo.lock file not found.", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error parsing Cargo.lock: {e}", file=sys.stderr)
        sys.exit(1)

    packages = data.get("package", [])
    package_versions = {}
    found_disallowed_crates = set()

    for pkg in packages:
        name = pkg.get("name")
        version = pkg.get("version")

        # Check for disallowed crates
        if name in DISALLOWED_CRATES:
            found_disallowed_crates.add(name)

        # Collect versions for each crate
        if name not in package_versions:
            package_versions[name] = set()
        package_versions[name].add(version)

    duplicates = {
        name: versions
        for name, versions in package_versions.items()
        if name in REQUIRE_SINGLE_VERSION and len(versions) > 1
    }

    exit_code = 0

    if duplicates:
        print("Error: The following crates are used with multiple versions:")
        for name, versions in duplicates.items():
            versions_list = ", ".join(sorted(versions))
            print(f" - {name}: versions {versions_list}")
        exit_code = 1

    if found_disallowed_crates:
        print("\nError: The following disallowed crates are used:")
        for name in sorted(found_disallowed_crates):
            print(f" - {name}")
        exit_code = 1

    if exit_code == 0:
        print(
            f"SUCCESS: only a single version of crates {REQUIRE_SINGLE_VERSION} is used, "
            + f"and no disallowed crates {DISALLOWED_CRATES} are used in Cargo.lock"
        )
    else:
        sys.exit(exit_code)


if __name__ == "__main__":
    main()
