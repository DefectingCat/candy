FROM rust as builder
WORKDIR /app

# Create appuser
ENV USER=candy
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

COPY . .

RUN mkdir $HOME/.cargo \
    && echo "[source.crates-io]" >> $HOME/.cargo/config \
    && echo "replace-with = 'ustc'" >> $HOME/.cargo/config \
    && echo "" >> $HOME/.cargo/config \
    && echo "[source.ustc]" >> $HOME/.cargo/config \
    && echo "registry = \"sparse+https://mirrors.ustc.edu.cn/crates.io-index/\"" >> $HOME/.cargo/config \
    && rustup target add x86_64-unknown-linux-musl

RUN update-ca-certificates

RUN cargo build --target x86_64-unknown-linux-musl --release

FROM scratch
 
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/candy /
COPY --from=builder /app/config.json /
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

USER candy

CMD ["/candy", "-c", "/config.json"]
