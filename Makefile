CARGO = cargo
RUSTC = rustc

all: release

build:
	$(CARGO) build

release:
	$(CARGO) build --release

dev:
	$(CARGO) watch -x run

run:
	$(CARGO) run

test:
	$(CARGO) test -- --test-threads=1

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

.PHONY: all
