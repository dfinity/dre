#!/usr/bin/env bash

command -v cargo-deny >/dev/null || echo "'cargo-deny' not found. Please install it by running 'cargo install cargo-deny'"
cargo deny check --warn unmaintained
