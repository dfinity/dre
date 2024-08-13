#!/usr/bin/env bash

TARGETS=(
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
)

echo "Will compile dre to following targets: ${TARGETS[@]}"
for target in "${TARGETS[@]}"; do
    echo "Compiling to: $target"
    cargo zigbuild --bin dre --release --target $target
    if [ $# -gt 0 ]; then
        SRC="target/$target/release/dre"
        DST="$1/dre-$target"
        echo "Copying $SRC -> $DST"
        cp $SRC $DST
    fi
done
