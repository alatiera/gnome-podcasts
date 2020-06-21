#! /usr/bin/sh

set -o errexit
set -o pipefail
set -x

# $1 Passed by meson and should be the builddir
export CARGO_TARGET_DIR="$1/target/"

# If this is run inside a flatpak envrironment, append the export the rustc
# sdk-extension binaries to the path
if [ -f "/.flatpak-info" ]
then
    export PATH="$PATH:/usr/lib/sdk/rust-stable/bin"
    # This assumes its run inside a Builder terminal
    export CARGO_TARGET_DIR="$BUILDDIR/target/"
fi

export CARGO_HOME="$CARGO_TARGET_DIR/cargo-home"

cargo test -- --test-threads=1 --nocapture
