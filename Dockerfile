################################################################################
# 阶段1: 构建阶段 - 使用官方Rust Alpine基础镜像进行编译
################################################################################
FROM rust:alpine AS builder

# 设置构建参数
ARG TARGET=aarch64-unknown-linux-musl
ARG BUILD_FLAGS="--release"

# 安装构建依赖（Alpine需要安装musl-dev和其他必要的编译工具）
RUN apk add --no-cache \
    musl-dev \
    gcc \
    g++ \
    make \
    openssl-dev \
    git

# 设置工作目录
WORKDIR /app

# 复制Cargo配置文件
COPY Cargo.toml Cargo.lock ./

RUN rustup target add aarch64-unknown-linux-musl

# 构建依赖（这将创建一个虚拟项目以便后续缓存依赖）
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs && \
    cargo build ${BUILD_FLAGS} --target ${TARGET} && \
    rm -rf src && \
    rm -f target/${TARGET}/release/deps/candy*

# 复制源代码
COPY src/ ./src/
COPY build.rs ./

# 构建项目 - 使用标准的musl链接
RUN cargo build ${BUILD_FLAGS} --target ${TARGET}

################################################################################
# 阶段2: 运行阶段 - 使用scratch基础镜像（最小化镜像）
################################################################################
FROM scratch

ARG TARGET=aarch64-unknown-linux-musl
# 从构建阶段复制编译好的二进制文件
COPY --from=builder /app/target/${TARGET}/release/candy /candy

# 复制配置文件
COPY config.toml /config.toml

EXPOSE 8080
EXPOSE 8081

# 入口点 - 启动Candy服务器
ENTRYPOINT ["/candy"]
CMD ["--config", "/config.toml"]
