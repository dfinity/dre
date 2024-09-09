#!/usr/bin/env bash

command -v cargo >/dev/null || {
    if test -x "$HOME/.cargo/bin/cargo"; then
        export PATH="$HOME/.cargo/bin:$PATH"
    else
        echo "'cargo' not found. Please install it by following the instructions at https://doc.rust-lang.org/cargo/getting-started/installation.html"
        exit 1
    fi
}

command -v cargo-deny >/dev/null || echo "'cargo-deny' not found. Please install it by running 'cargo install cargo-deny'"
# Do not change -D here.
# If there is a warning that causes a problem, and there
# is no fix at hand, then add an exception to deny.toml.
# If --warn unmaintained is added below, then the exceptions
# already listed in deny.toml are ignored, which is exactly
# the OPPOSITE of what we want.
cargo deny check -D warnings
