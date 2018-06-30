#!/usr/bin/sh

file="/.flatpak-info"
if [ -f "$file" ]
then
    # If you open a build terminal with gnome-builder
    source /usr/lib/sdk/rust-stable/enable.sh
    export CARGO_HOME=target/cargo-home/
fi

export RUSTFLAGS="--cfg rayon_unstable"
export RUSTBACKTRACE="1"

rustc --version
cargo --version
# cargo fmt --version

cargo build --all && \
cargo test -- --test-threads=1 && \
cargo test -- --test-threads=1 --ignored

# Rustfmt from the flatpak bundle tends to be outdated,
# will probably get better when rustfmt stabilizes
# cargo fmt --all -- --write-mode=diff

# cargo bench large
