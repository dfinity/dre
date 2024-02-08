set -x
function bazel_build_auto_repin() {
    local MYTMPDIR="$(mktemp -d)"
    trap 'rm -rf -- "$MYTMPDIR"' EXIT
    local logfile="$MYTMPDIR/bazel-out.log"
    # Build and save output but output to stderr in real time
    # so we can correct problems much faster than having
    # to wait until it's over building
    bazel "$@" 2>&1 | tee "$logfile" >&2
    local r=${PIPESTATUS[0]}  # Capture exit status of bazel
    if [ "$r" != "0" ]; then
        if grep -q 'Digests do not match' "$logfile" ; then
            # Repin necessary; only apply to this build
            CARGO_BAZEL_REPIN=true bazel "$@" || return $?
        else
            # Other failure, let's make like a tree and leave
            return $r
        fi
    fi
}

# Actual high-level call
bazel_build_auto_repin build ...
