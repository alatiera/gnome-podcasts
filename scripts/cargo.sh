#!/bin/sh

export CARGO_HOME=$1/target/cargo-home

cargo build --release -p hammond-gtk && cp $1/target/release/hammond-gtk $2