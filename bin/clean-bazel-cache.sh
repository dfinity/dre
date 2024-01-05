#!/usr/bin/env bash
#
# Clean up everything in CACHE_DIR last modified more than DAYS days ago.
#

set -eEuo pipefail

readonly CACHE_DIR="${1-${HOME}/.cache/bazel}"
readonly DAYS="${2-7}"
readonly dirlist="$(mktemp)"

echo "Ensuring subdirectories in ${CACHE_DIR} are writable"
find "${CACHE_DIR}" -type d -a \! -writable -exec chmod +w '{}' +

echo "Cleaning files in ${CACHE_DIR} with mtime >${DAYS} days"
find "${CACHE_DIR}" -mtime "+${DAYS}" \( -type f -o -type l \) -exec rm -vf '{}' +

echo -n "Cleaning empty directories..."
find "${CACHE_DIR}" -mindepth 1 -type d -empty -exec rmdir -v '{}' +

rm "${dirlist}"
