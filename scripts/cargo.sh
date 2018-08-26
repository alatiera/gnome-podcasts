#!/bin/sh

export CARGO_HOME=$1/target/cargo-home
export RUSTFLAGS="--cfg rayon_unstable"
export LOCALEDIR="$3"
export APP_ID="$4"
export VERSION="$5"
export PROFILE="$6"

if [[ "$PROFILE" == "Devel" ]]
then
    echo "DEBUG MODE"
    cargo build -p podcasts-gtk && cp $1/target/debug/podcasts-gtk $2
else
    echo "RELEASE MODE"
    cargo build --release -p podcasts-gtk && cp $1/target/release/podcasts-gtk $2
fi