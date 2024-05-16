#!/bin/bash

set -xue

rm -rf ./target/release/
rm -rf ./target/debug/
cargo build --release --target x86_64-unknown-linux-gnu

rm -rf ./target/release/
rm -rf ./target/debug/
cargo build --release --target x86_64-unknown-linux-musl

rm -rf ./target/release/
rm -rf ./target/debug/
cross build --release --target aarch64-unknown-linux-gnu

rm -rf ./target/release/
rm -rf ./target/debug/
cross build --release --target aarch64-unknown-linux-musl

rm -rf ./target/release/
rm -rf ./target/debug/
cross build --release --target x86_64-pc-windows-gnu

rm -rf ./target/release/
rm -rf ./target/debug/
cross build --release --target x86_64-unknown-freebsd
