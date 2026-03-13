#!/usr/bin/env bash

set -euo pipefail

# This script is called by Bazel to ensure requirements.lock is up to date
# It generates requirements.lock from uv.lock if needed

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

# Check if uv is available
if ! command -v uv &> /dev/null; then
    echo "Warning: uv is not installed or not in PATH" >&2
    echo "requirements.lock may be out of date" >&2
    exit 0
fi

# Check if uv.lock exists
if [ ! -f "uv.lock" ]; then
    echo "Warning: uv.lock not found" >&2
    exit 0
fi

# Check if requirements.lock exists and is newer than uv.lock
if [ -f "requirements.lock" ] && [ "requirements.lock" -nt "uv.lock" ]; then
    # requirements.lock is up to date
    exit 0
fi

# Generate requirements.lock from uv.lock
echo "Generating requirements.lock from uv.lock..." >&2
uv export --format requirements-txt --output-file requirements.lock
echo "Updated requirements.lock" >&2