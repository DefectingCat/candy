---
slug: cross-compile
title: Cross-Platform Build
authors: xfy
tags: [Rust]
---

Most Linux software is dynamically linked. Dynamic linking allows our programs to avoid bundling all required libraries. However, this has disadvantages - when the target system is old or lacks the required libraries, our binaries won't run.

## Installing Cross-Compilation Tools

We need not only to install the corresponding targets using `rustup` but also linkers to help build for target platforms.

macOS

```
brew tap SergioBenitez/osxct
brew install FiloSottile/musl-cross/musl-cross
brew install SergioBenitez/osxct/x86_64-unknown-linux-gnu
brew install mingw-w64
```

```
rustup target add x86_64-unknown-linux-musl
rustup target add x86_64-unknown-linux-gnu
rustup target add x86_64-pc-windows-gnu
rustup target add aarch64-unknown-linux-musl
rustup target add aarch64-unknown-linux-gnu
```

<!-- truncate -->

Ubuntu

```bash
apt-get install -y musl-tools libssl-dev pkg-config libudev-dev
```

## Dynamic Linking

```
TARGET_CC=x86_64-unknown-linux-gnu \
cargo build --release --target x86_64-unknown-linux-gnu
```

## Static Linking

```
TARGET_CC=x86_64-linux-musl-gcc \
RUSTFLAGS="-C linker=x86_64-linux-musl-gcc" \
cargo build --target=x86_64-unknown-linux-musl --release
```

```
TARGET_CC=x86_64-linux-musl-gcc \
cargo build --target=x86_64-unknown-linux-musl --release
```

## Windows

```
cargo build --target=x86_64-pc-windows-gnu --release
```

## Arm Linux

The A53 on my Raspberry Pi 3B+ is actually 64-bit, and the official system has supported 64-bit for a long time. If we compile directly on A53, it would be a waste.

Compiling to Arm Linux is similar to compiling to x86 Linux - just install the corresponding target and set the correct CC.

```bash
TARGET_CC=aarch64-linux-musl-cc \
RUSTFLAGS="-C linker=aarch64-linux-musl-cc" \
cargo build --target=aarch64-unknown-linux-musl --release
```

## Should I Rust or Should I Go?

Golang is more convenient:

```bash
GOOS=linux GOARCH=arm64 go build
```
