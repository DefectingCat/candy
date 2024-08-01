CARGO = cargo
RUSTC = rustc
CROSS = cross

all: build

build:
	$(CARGO) build

release: clean
	$(CARGO) build --release

dev:
	CANDY_LOG=debug $(CARGO) watch -x run

run:
	$(CARGO) run

test:
	$(CARGO) test

clean:
	$(CARGO) clean

clean-release:
	rm -rf ./target/release/
	rm -rf ./target/debug/

check:
	$(CARGO) check

format:
	$(CARGO) fmt

lint:
	$(CARGO) clippy

fix:
	$(CARGO) fix --allow-dirty --all-features

build-linux-musl: clean-release
	$(CROSS) build --release --target x86_64-unknown-linux-musl

build-linux-gnu: clean-release
	$(CROSS) build --release --target x86_64-unknown-linux-gnu

build-windows-gnu: clean-release
	$(CROSS) build --release --target x86_64-pc-windows-gnu

build-freebsd: clean-release
	$(CROSS) build --release --target x86_64-unknown-freebsd

build-loongarch: clean-release
	$(CROSS) build --release --target loongarch64-unknown-linux-gnu

.PHONY: all
