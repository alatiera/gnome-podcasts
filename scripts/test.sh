#! /bin/bash

set -o errexit
set -o pipefail
set -x

# $1 Passed by meson and should be the builddir
export CARGO_TARGET_DIR="$1/target/"
export CARGO_HOME="$CARGO_TARGET_DIR/cargo-home"

# If this is run inside a flatpak envrironment, append the export the rustc
# sdk-extension binaries to the path
if [ -f "/.flatpak-info" ]
then
    export PATH="$PATH:/usr/lib/sdk/rust-stable/bin"
fi

cargo fetch --locked
if command -v cargo-nextest &> /dev/null; then
    # nextest puts each [test] in its own process so they get their own
    # database and can run in parallel
    cargo nextest run --offline --no-capture --no-fail-fast
else
    cargo test --offline -- --test-threads=1 --nocapture
fi
