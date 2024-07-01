# Define the Rust compiler and cargo command
CARGO = cargo
RUSTC = rustc
CROSS = cross

# Targets
# TARGET = candy

# Default target to build the project
all: build

# Build the project
build:
	$(CARGO) build

build-release: clean
	$(CARGO) build --release

dev:
	CANDY_LOG=debug $(CARGO) watch -x run

# Run the project
run:
	$(CARGO) run

# Test the project
test:
	$(CARGO) test

# Clean the project
clean:
	$(CARGO) clean

clean-release:
	rm -rf ./target/release/
	rm -rf ./target/debug/

# Check the code for warnings and errors
check:
	$(CARGO) check

# Format the code using rustfmt
format:
	$(CARGO) fmt

# Clippy for linting
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

# Phony targets to avoid conflicts with file names
.PHONY: all build dev run test clean check format lint fix build-linux-musl build-linux-gnu build-windows-gnu build-freebsd build-loongarch
