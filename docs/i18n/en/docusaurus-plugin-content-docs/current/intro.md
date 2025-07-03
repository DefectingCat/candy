---
sidebar_position: 1
---

# Introduction

Candy is a lightweight HTTP server, aiming to quickly deploy a high-performance HTTP server.

## Installation

TODO

## Usage

Candy supports single executable file running:

```bash
❯ ./target/release/candy -h
Usage: candy [OPTIONS]

Options:
  -c, --config <FILE>  Sets a custom config file [default: ./config.toml]
  -h, --help           Print help
  -V, --version        Print version
```

Only one config file is supported, the default config file is `./config.toml`.

```bash
❯ ./target/release/candy -c config.toml
```

`-c` can be omitted, and when omitted, the default config file is `./config.toml` in the current directory.
