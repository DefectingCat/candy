# Define the Rust compiler and cargo command
CARGO = cargo
RUSTC = rustc

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
	$(CARGO) watch -x run

# Run the project
run:
	$(CARGO) run

# Test the project
test:
	$(CARGO) test

# Clean the project
clean:
	$(CARGO) clean

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
	cargo fix --allow-dirty --all-features

# Phony targets to avoid conflicts with file names
.PHONY: all build dev run test clean check format lint fix
