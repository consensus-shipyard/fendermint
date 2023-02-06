.PHONY: all build test lint license check-fmt check-clippy actor-bundle

BUILTIN_ACTOR_BUNDLE:=../builtin-actors/output/bundle.car

all: test build

build:
	cargo build --release

# Using --release for testing because wasm can otherwise be slow.
test: $(BUILTIN_ACTOR_BUNDLE)
	BUILTIN_ACTOR_BUNDLE=../../$(BUILTIN_ACTOR_BUNDLE) cargo test --release

clean:
	cargo clean

lint: \
	license \
	check-fmt \
	check-clippy

license:
	./scripts/add_license.sh

check-fmt:
	cargo fmt --all --check

check-clippy:
	cargo clippy --all -- -D warnings

# Build a bundle CAR; this is so we don't have to have a project reference,
# which means we are not tied to the release cycle of both FVM _and_ actors;
# so long as they work together.
actor-bundle: $(BUILTIN_ACTOR_BUNDLE)

$(BUILTIN_ACTOR_BUNDLE):
	cd .. && \
	if [ ! -d builtin-actors ]; then git clone https://github.com/filecoin-project/builtin-actors.git; fi && \
	cd builtin-actors && \
	git checkout next && \
	git pull && \
	cargo run --release --features "m2-native" -- -o output/$(shell basename $@)
