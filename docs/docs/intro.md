---
sidebar_position: 1
---

# 介绍

Candy 是一个轻量级的 HTTP 服务器，旨在快速部署一个高性能的 HTTP 服务器。

## 安装

TODO

## 使用

Candy 支持单个可执行文件运行：

```bash
❯ ./target/release/candy -h
Usage: candy [OPTIONS]

Options:
  -c, --config <FILE>  Sets a custom config file [default: ./config.toml]
  -h, --help           Print help
  -V, --version        Print version
```

只需要一个可执行文件和一个配置文件，就可以快速部署一个 HTTP 服务器。

```bash
❯ ./target/release/candy -c config.toml
```

`-c` 可以省略，当省略时，默认使用当前目录下的 `config.toml` 文件。
