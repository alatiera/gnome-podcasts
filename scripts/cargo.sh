#!/bin/bash

set -ex

export CARGO_HOME=$1/target/cargo-home
export LOCALEDIR="$3"
export APP_ID="$4"
export VERSION="$5"
export PROFILE="$6"

TARGET=debug
ARGS=()

if test "$PROFILE" != "Devel"; then
    echo "RELEASE MODE"
    ARGS+=('--release')
    TARGET=release
fi

if test -d vendor; then
    echo "VENDORED"
    ARGS+=('--frozen')
fi

cargo build ${ARGS[@]} -p podcasts-gtk && cp $1/target/${TARGET}/podcasts-gtk $2