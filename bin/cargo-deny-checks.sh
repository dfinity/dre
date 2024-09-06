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
cargo deny check
