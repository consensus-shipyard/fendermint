all: build
.PHONY: all

build:
	cargo build
.PHONY: build

clean:
	cargo clean

lint: license clean
	cargo fmt --all --check
	cargo clippy --all -- -D warnings

license:
	./scripts/add_license.sh

test:
	cargo test
