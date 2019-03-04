#! /usr/bin/sh

set -o errexit
set -o nounset
set -o pipefail

export CARGO_HOME="target/cargo-home"
export CARGO_TARGET_DIR="target_test/"

cargo test -j 1 -- --test-threads=1 --nocapture
