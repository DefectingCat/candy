---
slug: cross-compile
title: 跨平台构建
authors: xfy
tags: [Rust]
---

大多数 Linux 软件都是动态链接的，动态链接可以使我们的程序不需要将所有所需要的库都打包到自身当中去。不过这样也有弊处，当目标系统比较旧，或者压根就没有我们需要的库的时候，我们的二进制文件就无法在目标系统中运行。

## 安装交叉编译工具

不光光需要使用 `rustup` 来安装对应的 target，还需要 linker 来帮助构建到目标平台。

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

## 动态链接

```
TARGET_CC=x86_64-unknown-linux-gnu \
cargo build --release --target x86_64-unknown-linux-gnu
```

## 静态链接

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

手上的树莓派 3B+ 的 A53 其实是 64 位的，很久之前官方的系统就支持 64 位了。到现在一直都没有好好的发光发热过，如果使用 A53 来编译岂不是暴殄天物。

和编译到 x86 的 linux 一样，只要安装好对应的 target 和设置好对应的 CC 即可。

```bash
TARGET_CC=aarch64-linux-musl-cc \
RUSTFLAGS="-C linker=aarch64-linux-musl-cc" \
cargo build --target=aarch64-unknown-linux-musl --release
```

## Should I rust or should I go?

Golang 则更加的省心

```bash
GOOS=linux GOARCH=arm64 go build
```
