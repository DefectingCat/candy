CARGO = cargo
RUSTC = rustc
CROSS = CROSS_REMOTE=1 cross

all: build

build:
	$(CARGO) build

release:
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
	$(CARGO) fix --allow-dirty --all-features && $(CARGO) fmt

linux-musl: clean-release
	$(CROSS) build --release --target x86_64-unknown-linux-musl

linux-gnu: clean-release
	$(CROSS) build --release --target x86_64-unknown-linux-gnu

windows-gnu: clean-release
	$(CROSS) build --release --target x86_64-pc-windows-gnu

freebsd: clean-release
	$(CROSS) build --release --target x86_64-unknown-freebsd

loongarch: clean-release
	$(CROSS) build --release --target loongarch64-unknown-linux-gnu

.PHONY: all
