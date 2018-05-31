#!/bin/sh

export CARGO_HOME=$1/target/cargo-home
export RUSTFLAGS="--cfg rayon_unstable"

if [[ $DEBUG = true ]]
then
    echo "DEBUG MODE"
    cargo build -p hammond-gtk && cp $1/target/debug/hammond-gtk $2
else
    echo "RELEASE MODE"
    cargo build --release -p hammond-gtk && cp $1/target/release/hammond-gtk $2
fi 