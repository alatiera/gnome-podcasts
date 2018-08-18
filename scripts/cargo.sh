#!/bin/sh

export CARGO_HOME=$1/target/cargo-home
export RUSTFLAGS="--cfg rayon_unstable"
export PODCASTS_LOCALEDIR="$3"

if [[ $DEBUG = true ]]
then
    echo "DEBUG MODE"
    cargo build -p podcasts-gtk && cp $1/target/debug/podcasts-gtk $2
else
    echo "RELEASE MODE"
    cargo build --release -p podcasts-gtk && cp $1/target/release/podcasts-gtk $2
fi