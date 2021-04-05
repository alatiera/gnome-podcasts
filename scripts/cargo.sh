#!/bin/bash

set -ex

export MESON_BUILD_ROOT="$1"
export MESON_SOURCE_ROOT="$2"
export OUTPUT="$3"
export PROFILE="$4"
export CARGO_TARGET_DIR="$MESON_BUILD_ROOT"/target
export CARGO_HOME="$MESON_BUILD_ROOT"/cargo-home

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

cargo build ${ARGS[@]} --manifest-path="$MESON_SOURCE_ROOT"/Cargo.toml -p podcasts-gtk
cp "$CARGO_TARGET_DIR"/${TARGET}/podcasts-gtk "$OUTPUT"
