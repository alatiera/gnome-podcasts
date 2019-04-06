#! /usr/bin/sh

set -o errexit
set -o pipefail

export CARGO_TARGET_DIR="$1/target/"

# If this is run inside a flatpak envrironment, append the export the rustc
# sdk-extension binaries to the path
if [ -n "/.flatpak-info" ]
then
    export PATH="$PATH:/usr/lib/sdk/rust-stable/bin"
    export CARGO_TARGET_DIR="$BUILDDIR/target/"
fi

export CARGO_HOME="$CARGO_TARGET_DIR/cargo-home"

cargo test -j 1 -- --test-threads=1 --nocapture
