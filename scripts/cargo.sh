#!/bin/bash

set -ex

export OUTPUT="$2"
export CARGO_TARGET_DIR="$3"/target
export CARGO_HOME="$CARGO_TARGET_DIR"/cargo-home
export PROFILE="$4"

TARGET=debug
ARGS=()

if test "$PROFILE" != ".Devel"; then
    echo "RELEASE MODE"
    ARGS+=('--release')
    TARGET=release
fi

if test -d vendor; then
    echo "VENDORED"
    ARGS+=('--frozen')
fi

cargo build ${ARGS[@]} --manifest-path="$1"/Cargo.toml -p podcasts-gtk
cp "$CARGO_TARGET_DIR"/${TARGET}/podcasts-gtk "$OUTPUT"
