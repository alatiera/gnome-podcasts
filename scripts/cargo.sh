#!/bin/sh

cargo build --release -p hammond-gtk && cp $1/target/release/hammond-gtk $2
